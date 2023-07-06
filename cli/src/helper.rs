use rustyline::{
    completion::{Completer, Pair},
    highlight::Highlighter,
    hint::Hinter,
    validate::Validator,
    Helper,
};

#[derive(Helper)]
pub struct CliHelper {
    commands: Vec<String>,
}

impl CliHelper {
    pub fn new(commands: Vec<String>) -> CliHelper {
        CliHelper { commands }
    }
}

impl Completer for CliHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        let mut candidates: Vec<Pair> = self
            .commands
            .iter()
            .filter(|c| c.starts_with(&line[..pos]))
            .map(|c| Pair {
                display: c[pos..].to_owned(),
                replacement: c[pos..].to_owned(),
            })
            .collect();

        let prefix =
            rustyline::completion::longest_common_prefix(&candidates).map(|s| s.to_owned());
        if let Some(prefix) = prefix {
            if prefix != "" {
                candidates.clear();
                candidates.push(Pair {
                    display: prefix.clone(),
                    replacement: prefix,
                });
            }
        }

        Ok((pos, candidates))
    }
}

impl Validator for CliHelper {}

impl Hinter for CliHelper {
    type Hint = String;
}

impl Highlighter for CliHelper {}
