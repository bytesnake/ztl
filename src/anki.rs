use crate::config::{self, Config};
use std::fs;
use std::collections::HashMap;
use crate::{notes, utils};
use anyhow::{Result, Context};

use genanki_rs::{Field, Deck, Note, Model, Template, Error};

pub(crate) fn ankify(config: Config, target: &str) -> Result<()> {
    let css = r#"
.card {
 font-family: arial;
 font-size: 16px;
 color: black;
 background-color: white;
 width: 380px;
 height: 100%;
  margin-left: auto;
  margin-right: auto;
}
mspace{margin-left:0.17em;}
.note {
  border-left: 2px solid gray;
  padding: 0px 0px 0 5px;
}

.newtheorem p {
margin: 0;
}

.head {
display:block;
margin-bottom: 5px;
}
dl {
  display: grid;
  grid-template-columns: max-content auto;
}

dt {
  grid-column-start: 1;
}

dd {
  grid-column-start: 2;
}
    "#;

    let published_path = config::get_config_path()
        .parent().unwrap().join("published");

    let mut hash: HashMap<String, (String, String)> = fs::read_to_string(&published_path)
        .map(|x| toml::from_str(&x).unwrap())
        .unwrap_or(HashMap::new());

    let mut notes = notes::Notes::from_cache().notes;

    let proof_model = Model::new(
        1607392317,
        "Model for theorem proofs",
        vec![Field::new("parent"), Field::new("note"), Field::new("modifier"), Field::new("address")],
        vec![Template::new("Proof of theorem")
            .qfmt(r#"<style>#{{modifier}} { background: repeating-linear-gradient( -45deg, #e35336, #e35336 2px, white 2px, white 10px); color: transparent; }</style><a class='address' href='https://zettel.haus/@losch/{{address}}'>Note</a><br /><br /><div class="note">{{parent}}</div><br /><div class="note">{{note}}</div>"#)
            .afmt(r#"<style>#{{modifier}} { color: #e35336; }</style><a class='address' href='https://zettel.haus/@losch/{{address}}'>Note</a><br /><br /><div class="note">{{parent}}</div><br /><div class="note">{{note}}</div>"#)],
    ).css(&css);

    let proof_assump_model = Model::new(
        1607392318,
        "Model for theorem proofs, assumptions",
        vec![Field::new("parent"), Field::new("note"), Field::new("modifier"), Field::new("address")],
        vec![Template::new("Consequence of assumption")
            .qfmt(r#"<style>.assumption#{{modifier}} { color: #e35336; }</style><a class='address' href='https://zettel.haus/@losch/{{address}}'>Note</a> What happens if the assumption fails?<br /><br /><div class="note">{{parent}}</div><br /><div class="note">{{note}}</div>"#)
            .afmt(r#"<style>#{{modifier}} { color: #e35336; }</style><a class='address' href='https://zettel.haus/@losch/{{address}}'>Note</a><br /><br /><div class="note">{{parent}}</div><br /><div class="note">{{note}}</div>"#)],
    ).css(&css);

    let mut my_deck = Deck::new(
        2059400114,
        "Notes export",
        "Deck for studying ztl",
    );

    let mut ncards = 0;
    for (key,note) in &notes {
        if note.cards.is_empty() || note.kind.as_ref().map(|x| x != "proof").unwrap_or(false) {
            continue;
        }

        let parent = note.parent.as_ref().map(|x| notes.get(x).unwrap().html.clone()).unwrap_or(String::new());
        let address = hash.get(key).map(|x| x.1.clone()).unwrap_or("".to_string());

        for card in &note.cards {
            let note = match card {
                notes::Card::Cloze { target, .. } => {
                    let hash = utils::hash(&format!("{}{}", target, note.id));
                    Note::new(proof_model.clone(), vec![
                        &parent,
                        &note.html,
                        &target,
                        &address,
                    ])?.guid(hash)
                },
                notes::Card::Assumption { target } => {
                    let hash = utils::hash(&format!("{}{}", target, note.id));
                    Note::new(proof_assump_model.clone(), vec![
                        &parent,
                        &note.html,
                        &target,
                        &address,
                    ])?.guid(hash)
                },
            };

            my_deck.add_note(note);
            ncards += 1;
        }
    }

    my_deck.write_to_file(target)?;
    println!("Written {} cards to {}", ncards, target);

    Ok(())
}
