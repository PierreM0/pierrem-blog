use std::{
    collections::HashMap,
    fs::File,
    path::{Path, PathBuf}, io::{BufReader, Write, Read},
};

use crate::error::{Error, Result};
use priority_queue::PriorityQueue;
use rust_stemmers::{self, Algorithm, Stemmer};
use serde::{Deserialize, Serialize};
use tracing::warn;

#[derive(Serialize, Deserialize, Clone)]
struct Document {
    word_count: HashMap<String, usize>,
    lenght: i32,
}
impl Document {
    const fn new(word_count: HashMap<String, usize>, lenght: i32) -> Self {
        Self { word_count, lenght }
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct Global {
    word_count: HashMap<String, Document>,
    lenght: i32,
}
impl Global {
    const fn new(word_count: HashMap<String, Document>, lenght: i32) -> Self {
        Self { word_count, lenght }
    }
}

pub fn search(search_query: Vec<String>) -> Result<PriorityQueue<String, i32>> {
    let en_stemmer = Stemmer::create(Algorithm::English);
    let search_query = search_query
        .iter()
        .map(|s| en_stemmer.stem(s.to_lowercase().as_str()).to_string())
        .collect::<Vec<_>>();

    let scanned_documents = scan_path(crate::get_article_path()?)?;

    let mut priority_queue = PriorityQueue::<String, i32>::new();

    for (document_name, document) in &scanned_documents.word_count {
        let score = score(document, &search_query, &scanned_documents);
        priority_queue.push(document_name.to_owned(), score);
    }

    Ok(priority_queue)
}

fn words_time_by_documents(document: &str) -> Document {
    let mut word_count = HashMap::new();
    let document = document.split_whitespace();
    let mut counter = 0;
    let en_stemmer = rust_stemmers::Stemmer::create(Algorithm::English);
    for word in document {
        counter += 1;
        let word = en_stemmer.stem(&word.to_lowercase()).to_string();
        #[allow(clippy::map_entry)]
        if word_count.contains_key(&word) {
            if let Some(x) = word_count.get_mut(&word) {
                *x += 1;
            }
        } else {
            let _ = word_count.insert(word, 1);
        }
    }

    Document::new(word_count, counter)
}

fn scan_all_documents(dir: PathBuf, scanned_documents: &mut Global) -> Result<Global> {
    
    let dir = dir.read_dir().map_err(|_| Error::NotFound)?;

    let mut counter = 0;
    for file in dir {
        counter += 1;
        let Ok(file) = file else { continue };
        let Ok(file_type) = file.file_type() else {continue};
        if file_type.is_file() {
            let file_path = file.path();
            let Some(file_name) = file_path.to_str()
                .to_owned() else {continue};

            let Ok(file) = File::open(file_name) else { continue };

            let mut file_content = String::new();
            let mut buf_reader = BufReader::new(file);
            let _ = buf_reader.read_to_string(&mut file_content);

            if !scanned_documents
                .word_count
                .contains_key(file_name)
            {
                let doc = words_time_by_documents(&file_content);

                scanned_documents.word_count.insert(file_name.to_owned(), doc);
            }
        }
    }
    scanned_documents.lenght = counter;

    Ok(scanned_documents.clone())
}

fn scan_path(path: PathBuf) -> Result<Global> {
    let dir = path;
    let data_path = Path::new("data.json");

    let file = match File::open(data_path) {
        Ok(f) => Ok(f),
        Err(e) => {
            warn!("ERROR: {e}");
            let _ = File::create(data_path);
            File::open(data_path).map_err(|_| Error::IoError)
        }
    }?;

    let mut documents: Global = serde_json::from_reader(BufReader::new(file))
        .unwrap_or_else(|_| Global::new(HashMap::new(), 0));
    let scanned_documents = scan_all_documents(dir, &mut documents)?;
    let serialized = serde_json::to_string(&scanned_documents).unwrap_or_else(|_| String::new());

    let _ = File::create(data_path).map_err(|_| Error::IoError)?
        .write_all(serialized.as_bytes());

    Ok(scanned_documents)
}

fn idf(word: &str, global_documents: &Global) -> f32 {
    let mut document_containing_word = 0;
    for document in global_documents.word_count.values() {
        if document.word_count.contains_key(word) {
            document_containing_word += 1;
        }
    }
    let numerator = (global_documents.lenght - document_containing_word) as f32 + 0.5;
    let denominator = document_containing_word as f32 + 0.5;

    (1f32 + (numerator / denominator)).log10()
}

fn score(document: &Document, query: &Vec<String>, global_documents: &Global) -> i32 {
    let k1 = 1.2;
    let b = 0.75; // magic numbers
    let zero: usize = 0;

    let mut score: f32 = 0.0;

    let mut d_on_avgdl = 0;
    for doc in global_documents.word_count.values() {
        d_on_avgdl += doc.lenght;
    }
    let d_on_avgdl = d_on_avgdl as f32 / global_documents.lenght as f32;

    for word in query {
        let freq_word_document = document.word_count.get(word).unwrap_or(&zero).to_owned();
        let idf = idf(word, global_documents);
        let numerator = freq_word_document as f32 * (k1 + 1.0);
        let denominator = freq_word_document as f32 + k1.mul_add(b * d_on_avgdl, 1. - b);
        score += idf * (numerator / denominator);
    }
    (score * 1000.0).round() as i32 
}
