# Bloging engine

To start it :
```sh 
$ cargo run -- --notls --path=articles
```

then go to

`http://localhost:6969/`

Option :
- `notls` : disable rustls and run the engine as HTTP
- `path` : path to the website folder, there should be a subfolder for article named `articles`, one for images named `images`, and one for assets named `assets`

***

## More infos

### Titles
Your titles are the name of your markdown files, without the extention and with `_` replaced by spaces ` `
Ex: 
- `README.md` becomes `README`
- `I_really_like_flowers,_here_are_a_few_pictures.markdown' becomes 'I really like flower, here are a few pictures'

