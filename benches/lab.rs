use typst::parse::{is_newline, Scanner};

/// The comment prefix that indicates commands.
const PREFIX: &'static str = "//%% ";

/// What action a [`Command`] will perform.
#[derive(Debug, Clone, PartialEq)]
enum CommandKind {
    /// Insert the payload.
    Insert,
    /// Initially show the payload, then delete it.
    Delete,
    /// Replace the payload with the string.
    Replace(String),
}

impl CommandKind {
    /// Retrieve the secondary payload if the command has one.
    fn param<'s>(&'s self) -> Option<&'s str> {
        match self {
            Self::Insert | Self::Delete => None,
            Self::Replace(param) => Some(param),
        }
    }
}

/// Modifiers for the command.
#[derive(Debug, Clone, Copy, PartialEq)]
struct CommandParameters {
    /// A last step will be added where the area reverts to its initial state.
    undo: bool,
    /// The command will be executed one character at a time, as to simulate typing.
    typing: bool,
}

/// A command for modification over time in the source code.
#[derive(Debug, Clone, PartialEq)]
struct Command {
    /// What action to perform.
    kind: CommandKind,
    /// Where in the commandless source string the command data should be
    /// inserted.
    start: usize,
    /// Additional parameters.
    params: CommandParameters,
    /// The primary command payload.
    payload: String,
    /// The number of Unicode scalars in the command payload.
    payload_chars: usize,
}

impl Command {
    /// Create a new command.
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
            payload_chars: payload.chars().count(),
            payload,
        }
    }

    /// Whether the `undo` parameter is set.
    fn is_undo(&self) -> bool {
        self.params.undo
    }

    /// Whether the `typing` parameter is set.
    fn is_typing(&self) -> bool {
        self.params.typing
    }

    /// The total number of states this command can run through.
    fn states(&self) -> usize {
        let res = match (&self.kind, self.is_typing()) {
            (CommandKind::Replace(_), false) => 3,
            (_, false) => 2,
            (c, true) => self.payload_chars + c.param().map(str::len).unwrap_or(0) + 1,
        };

        if self.is_undo() { res + 1 } else { res }
    }

    /// Retrieve a particular state of the command.
    fn step(&self, step: usize) -> Option<String> {
        let mut res = None;

        match (&self.kind, self.is_typing()) {
            (CommandKind::Insert, true) if step <= self.payload_chars => {
                res = Some(self.payload.chars().take(step).collect());
            }
            (CommandKind::Insert, false) => {
                if step == 1 {
                    res = Some(self.payload.clone());
                } else if step == 0 {
                    res = Some(String::new());
                }
            }
            (CommandKind::Delete, true) if step <= self.payload_chars => {
                res =
                    Some(self.payload.chars().take(self.payload_chars - step).collect());
            }
            (CommandKind::Delete, false) => {
                if step == 1 {
                    res = Some(String::new());
                } else if step == 0 {
                    res = Some(self.payload.clone());
                }
            }
            (CommandKind::Replace(replace), true) => {
                let replace_count = replace.chars().count();

                if step <= self.payload_chars + replace_count {
                    res = Some(if step <= self.payload_chars {
                        self.payload.chars().take(self.payload_chars - step).collect()
                    } else {
                        let remaining = step - self.payload_chars;
                        replace.chars().take(remaining).collect()
                    });
                }
            }
            (CommandKind::Replace(replace), false) if step <= 2 => {
                res = Some(if step == 1 { String::new() } else { replace.clone() });
            }
            _ => {}
        };

        if self.is_undo() && step + 1 == self.states() {
            // Return the initial state.

            return Some(match self.kind {
                CommandKind::Insert => String::new(),
                CommandKind::Delete | CommandKind::Replace(_) => self.payload.clone(),
            });
        }

        return res;
    }

    /// Create an iterator for all the steps of the command.
    fn iter<'s>(&'s self) -> CommandIterator<'s> {
        CommandIterator::new(self)
    }
}

/// Iterator that allows to step through all states of a [`Command`].
#[derive(Debug, Copy, Clone, PartialEq)]
struct CommandIterator<'s> {
    /// The underlying command.
    command: &'s Command,
    /// The next step index when moving forward through the iterator.
    step: usize,
    /// The last step index when moving backwards through the iterator.
    step_back: usize,
    /// The total amount of steps in the iterator.
    len: usize,
}

impl<'s> CommandIterator<'s> {
    /// Create a new command iterator.
    fn new(command: &'s Command) -> Self {
        let len = command.states();
        Self { command, step: 0, step_back: len, len }
    }
}

impl<'s> Iterator for CommandIterator<'s> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        if self.step > self.step_back {
            return None;
        }

        let res = self.command.step(self.step);
        self.step += 1;
        res
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        let res = self.command.step(n);
        self.step = n + 1;
        res
    }
}

impl<'s> ExactSizeIterator for CommandIterator<'s> {}

impl<'s> DoubleEndedIterator for CommandIterator<'s> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.step > self.step_back {
            return None;
        }

        let res = self.command.step(self.step_back);
        self.step_back -= 1;
        res
    }
}

impl<'s> IntoIterator for &'s Command {
    type Item = String;
    type IntoIter = CommandIterator<'s>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Parsing and iterating through test files prepared with commands.
///
/// A laboratory will extract comments with special commands from a Typst file
/// and allow to run through the various command-defined states of the file.
/// It is intended for performance measurements.
#[derive(Debug, Clone, PartialEq)]
pub struct Lab {
    /// The source string with no commands.
    source: String,
    /// A list of commands.
    commands: Vec<Command>,
}

impl Lab {
    /// Parse a source string.
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

    /// Return an iterator for the various command states of the file.
    pub fn iter<'s>(&'s self) -> LabIterator<'s> {
        LabIterator::new(self)
    }
}

/// Iterate through the states of a [`Lab`], as defined by the commands.
#[derive(Debug, Clone, PartialEq)]
pub struct LabIterator<'s> {
    /// The underlying lab.
    lab: &'s Lab,
    /// Command iterators derived from the lab's commands.
    command_iterators: Vec<CommandIterator<'s>>,
    /// The highest step number each command is defined for.
    max_step: Vec<usize>,
    /// The current position of the iterator.
    step: usize,
}

impl<'s> LabIterator<'s> {
    /// Create a new iterator.
    fn new(lab: &'s Lab) -> Self {
        let command_iterators: Vec<_> = lab.commands.iter().map(Command::iter).collect();
        let steps = command_iterators.iter().map(|i| i.len() - 1).collect();
        Self {
            lab,
            command_iterators,
            max_step: steps,
            step: 0,
        }
    }
}

impl<'s> Iterator for LabIterator<'s> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        if self.step > self.max_step.iter().sum() {
            return None;
        }

        let mut available_steps = self.step;
        let mut offset = 0;
        let mut res = self.lab.source.clone();

        for (i, mut command_iter) in self.command_iterators.iter().copied().enumerate() {
            let position = self.lab.commands[i].start + offset;
            let steps = self.max_step[i].min(available_steps);

            let insertion = command_iter.nth(steps).unwrap();

            available_steps -= steps;
            offset += insertion.len();

            res.insert_str(position, &insertion);
        }

        self.step += 1;

        Some(res)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let res = self.max_step.iter().sum();
        (res, Some(res))
    }
}

impl<'s> ExactSizeIterator for LabIterator<'s> {}

impl<'s> IntoIterator for &'s Lab {
    type Item = String;
    type IntoIter = LabIterator<'s>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Parse a command.
fn command(s: &mut Scanner, start: usize) -> Option<Command> {
    if !command_prefix(s) {
        return None;
    }

    let command = ident(s).to_string();
    let mut params = vec![];

    // Get other command parameters
    while !s.eof() {
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
    while !s.eof() {
        if !command_prefix(s) {
            break;
        }

        secondary.push_str(until_newstart(s));
    }

    let mut payload = String::new();
    while !s.eof() {
        if command_prefix(s) {
            break;
        }

        payload.push_str(until_newstart(s));
    }

    let end_command = ident(s);
    if end_command.to_uppercase() != "END" {
        panic!("expected end command for {}", command);
    }
    s.eat_until(is_newline);
    s.eat();

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

/// Eat a command prefix. The function will return if the command prefix has
/// been found.
fn command_prefix(s: &mut Scanner) -> bool {
    if !s.rest().starts_with(PREFIX) {
        return false;
    }

    for _ in 0 .. PREFIX.len() {
        s.eat();
    }

    true
}

/// Return an identifier.
fn ident<'s>(s: &'s mut Scanner) -> &'s str {
    s.eat_while(char::is_alphabetic)
}

/// Eat the current line and continue until something other than newlines are
/// found.
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
