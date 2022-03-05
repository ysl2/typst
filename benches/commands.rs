use typst::parse::{is_newline, Scanner};

const PREFIX: &'static str = "//%% ";

#[derive(Debug, Clone, PartialEq)]
enum CommandKind {
    Insert,
    Delete,
    Replace(String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct CommandParameters {
    undo: bool,
    typing: bool,
}

#[derive(Debug, Clone, PartialEq)]
struct Command {
    kind: CommandKind,
    start: usize,
    params: CommandParameters,
    payload: String,
}

impl Command {
    fn new(
        kind: CommandKind,
        start: usize,
        payload: String,
        undo: bool,
        typing: bool,
    ) -> Self {
        Self {
            kind,
            start,
            params: CommandParameters { undo, typing },
            payload,
        }
    }

    fn is_undo(&self) -> bool {
        self.params.undo
    }

    fn is_typing(&self) -> bool {
        self.params.typing
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Lab {
    source: String,
    commands: Vec<Command>,
}

impl Lab {
    pub fn new(src: &str) -> Self {
        let mut source = String::with_capacity(src.len());
        let mut s = Scanner::new(src);
        let mut commands = vec![];

        while !s.eof() {
            source.push_str(until_newstart(&mut s));
            if let Some(command) = command(&mut s, source.len()) {
                commands.push(command);
            }
        }

        source.shrink_to_fit();
        Lab { source, commands }
    }
}

fn command(s: &mut Scanner, start: usize) -> Option<Command> {
    if !command_prefix(s) {
        return None;
    }

    let command = ident(s).to_string();
    let mut params = vec![];

    // Get other command parameters
    loop {
        s.eat_if(' ');
        let param = ident(s).to_string();
        if !param.is_empty() {
            params.push(param);
        } else {
            break;
        }
    }

    // Parse secondary payloads.
    until_newstart(s);
    let mut secondary = String::new();
    loop {
        if !command_prefix(s) {
            break;
        }

        secondary.push_str(until_newstart(s));
    }

    let mut payload = String::new();
    loop {
        if command_prefix(s) {
            break;
        }

        payload.push_str(until_newstart(s));
    }

    let end_command = ident(s);
    if end_command.to_uppercase() != "END" {
        panic!("expected end command for {}", command);
    }
    until_newstart(s);

    let kind = match command.to_uppercase().as_ref() {
        "INSERT" => CommandKind::Insert,
        "DELETE" => CommandKind::Delete,
        "REPLACE" => CommandKind::Replace(secondary),
        c => panic!("unknown command {}", c),
    };

    let undo = params.contains(&"undo".to_string());
    let typing = params.contains(&"typing".to_string());

    Some(Command::new(kind, start, payload, undo, typing))
}

fn command_prefix(s: &mut Scanner) -> bool {
    if !s.rest().starts_with(PREFIX) {
        return false;
    }

    for _ in 0 .. PREFIX.len() {
        s.eat();
    }

    true
}

fn ident<'s>(s: &'s mut Scanner) -> &'s str {
    s.eat_while(char::is_alphabetic)
}

fn until_newstart<'s>(s: &'s mut Scanner) -> &'s str {
    let mut at_start = false;
    s.eat_until(|c| {
        let newline = is_newline(c);

        if !newline && at_start {
            return true;
        }

        if newline {
            at_start = true;
        } else {
            at_start = false;
        }

        false
    })
}
