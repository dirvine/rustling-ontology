#[macro_use]
extern crate clap;
extern crate rustling_ontology;
#[macro_use]
extern crate prettytable;

use std::str::FromStr;

use rustling_ontology::*;
use prettytable::Table;

fn main() {
    let matches = clap_app!(rustling_cli =>
        (@arg lang: -l --lang default_value[en] "2-letter language code (default to \"en\")")
        (@subcommand parse =>
             (@arg kinds: -k --kinds +takes_value +use_delimiter "kinds, last one wins, coma separated")
             (@arg sentence: +required "Sentence to test")
        )
        (@subcommand play =>
             (@arg kinds: -k --kinds +takes_value +use_delimiter "kinds, last one wins, coma separated")
             (@arg sentence: +required "Sentence to test")
        )
    )
        .get_matches();

    let lang = value_t!(matches.value_of("lang"), Lang).unwrap_or_else(|e| e.exit());
    match matches.subcommand() {
        ("parse", Some(matches)) => {
            let kinds = matches
                  .values_of("kinds")
                  .map(|values| {
                           values
                               .map(|s| OutputKind::from_str(s).unwrap())
                               .collect::<Vec<_>>()
                  });
            let sentence = matches.value_of("sentence").unwrap().to_lowercase();
            let parser = build_parser(lang).unwrap();
            let context = ResolverContext::default();
            let entities = if let Some(kinds) = kinds {
                parser.parse_with_kind_order(&*sentence, &context, &kinds).unwrap()
            } else {
                parser.parse(&*sentence, &context).unwrap()
            };
            let mut table = Table::new();
            table.set_titles(row!["ix", "log(p)", "p", "text", "value"]);
            for (ix, c) in entities.iter().enumerate().rev() {
                let mut hilite = String::new();
                for _ in 0..c.byte_range.0 {
                    hilite.push('_');
                }
                hilite.push_str(&sentence[c.byte_range.0..c.byte_range.1]);
                for _ in c.byte_range.1..sentence.len() {
                    hilite.push('_');
                }
                table.add_row(row![ix,
                                   c.probalog,
                                   f32::exp(c.probalog),
                                   hilite,
                                   format!("{:?}", c.value)]);
            }
            table.printstd();
        }
        ("play", Some(matches)) => {
            let kinds = matches
                .values_of("kinds")
                .map(|values| {
                         values
                             .map(|s| OutputKind::from_str(s).unwrap())
                             .collect()
                     })
                .unwrap_or(OutputKind::all());
            let sentence = matches.value_of("sentence").unwrap().to_lowercase();
            let parser = build_raw_parser(lang).unwrap();
            
            let context = IdentityContext::new();
            let tagger = CandidateTagger {
                order: &kinds,
                context: &context,
                resolve_all_candidates: true,
            };
            let candidates = parser.candidates(&*sentence, &tagger).unwrap();
            let mut table = Table::new();
            table.set_format(*prettytable::format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
            table.set_titles(row!["ix", "best", "log(p)", "p", "text", "value", "latent", "rule", "childs"]);

            for (ix, c) in candidates.iter().enumerate().rev() {
                let mut hilite = String::new();
                for _ in 0..c.match_.byte_range.0 {
                    hilite.push('_');
                }
                hilite.push_str(&sentence[c.match_.byte_range.0..c.match_.byte_range.1]);
                for _ in c.match_.byte_range.1..sentence.len() {
                    hilite.push('_');
                }
                table.add_row(row![ix,
                                   if c.tagged { "*" } else { " " },
                                   c.match_.probalog,
                                   f32::exp(c.match_.probalog),
                                   hilite,
                                   c.match_.value.as_ref().map(|v| format!("{:?}", v)).unwrap_or("".into()),
                                   c.node.value.latent(),
                                   parser.resolve_sym(&c.node.root_node.rule_sym).unwrap_or(""),
                                   c.node
                                       .root_node
                                       .children
                                       .iter()
                                       .map(|n| {
                                           let name = parser.resolve_sym(&n.rule_sym).unwrap_or("");
                                                name.chars().take(20).collect::<String>()
                                            })
                                       .collect::<Vec<_>>()
                                       .join(" + ")]);
            }
            table.printstd();
        }
        (cmd, _) => panic!("Unknown command {}", cmd),
    }
}
