use std::ops::Range;
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
    /// The command will be executed one character at a time, as to simulate
    /// typing.
    typing: bool,
}

/// A change to a string, expressed by a range to replace with some content.
#[derive(Debug, Clone, PartialEq)]
pub struct Change {
    /// The range in the original string to be replaced.
    pub range: Range<usize>,
    /// What to replace the content in the range with.
    pub content: String,
}

impl Change {
    /// Create a new change.
    pub fn new(range: Range<usize>, content: String) -> Self {
        Self { range, content }
    }

    /// Create a new insertion at an index in the original string.
    pub fn insert(pos: usize, content: String) -> Self {
        Self { range: pos .. pos, content }
    }

    /// Delete the text in the range.
    pub fn clear(range: Range<usize>) -> Self {
        Self { range, content: String::new() }
    }

    /// Map the replacement range with some function.
    pub fn map_range<F>(&mut self, mut f: F)
    where
        F: FnMut(usize) -> usize,
    {
        self.range = f(self.range.start) .. f(self.range.end)
    }

    /// The total length delta the change causes.
    pub fn len(&self) -> isize {
        self.content.len() as isize - self.range.len() as isize
    }
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
            (CommandKind::Replace(_), false) => 2,
            (_, false) => 1,
            (c, true) => self.payload_chars + c.param().map(str::len).unwrap_or(0),
        };

        if self.is_undo() { res + 1 } else { res }
    }

    /// The initial state of the document when the command has not yet made a
    /// change.
    fn initial<'s>(&'s self) -> &'s str {
        match self.kind {
            CommandKind::Delete | CommandKind::Replace(_) => &self.payload,
            CommandKind::Insert => "",
        }
    }

    /// Retrieve a particular state of the command.
    fn step(&self, step: usize) -> Option<Change> {
        let mut res = None;

        match (&self.kind, self.is_typing()) {
            (CommandKind::Insert, true) if step < self.payload_chars => {
                let (offset, c) = self.payload.char_indices().nth(step).unwrap();
                let pos = self.start + offset;

                res = Some(Change::insert(pos, c.into()));
            }
            (CommandKind::Insert, false) if step == 0 => {
                res = Some(Change::insert(self.start, self.payload.clone()));
            }
            (CommandKind::Delete, true) if step < self.payload_chars => {
                let (idx, c) = self.payload.char_indices().rev().nth(step).unwrap();
                let pos = self.start + idx;

                res = Some(Change::clear(pos .. pos + c.len_utf8()));
            }
            (CommandKind::Delete, false) if step == 0 => {
                res = Some(Change::clear(self.start .. self.start + self.payload.len()))
            }
            (CommandKind::Replace(replace), true) => {
                let replace_count = replace.chars().count();

                if step < self.payload_chars + replace_count {
                    res = Some(if step < self.payload_chars {
                        let (idx, c) =
                            self.payload.char_indices().rev().nth(step).unwrap();
                        let pos = self.start + idx;

                        Change::clear(pos .. pos + c.len_utf8())
                    } else {
                        let remaining = step - self.payload_chars;
                        let (offset, c) = replace.char_indices().nth(remaining).unwrap();
                        let pos = self.start + offset;

                        Change::insert(pos, c.into())
                    });
                }
            }
            (CommandKind::Replace(replace), false) if step <= 1 => {
                res = Some(if step == 0 {
                    Change::clear(self.start .. self.start + self.payload.len())
                } else {
                    Change::insert(self.start, replace.clone())
                });
            }
            _ => {}
        };

        if self.is_undo() && step + 1 == self.states() {
            // Return the initial state.

            return Some(match &self.kind {
                CommandKind::Insert => {
                    Change::clear(self.start .. self.start + self.payload.len())
                }
                CommandKind::Delete => {
                    Change::new(self.start .. self.start, self.payload.clone())
                }
                CommandKind::Replace(r) => {
                    Change::new(self.start .. self.start + r.len(), self.payload.clone())
                }
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
    /// The total amount of steps in the iterator.
    len: usize,
}

impl<'s> CommandIterator<'s> {
    /// Create a new command iterator.
    fn new(command: &'s Command) -> Self {
        let len = command.states();
        Self { command, step: 0, len }
    }
}

impl<'s> Iterator for CommandIterator<'s> {
    type Item = Change;

    fn next(&mut self) -> Option<Self::Item> {
        if self.step >= self.len {
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

impl<'s> IntoIterator for &'s Command {
    type Item = Change;
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
                source.push_str(command.initial());
                commands.push(command);
            }
        }

        source.shrink_to_fit();
        Lab { source, commands }
    }

    /// Return a reference to the source, freed of commands.
    pub fn source<'s>(&'s self) -> &'s str {
        &self.source
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
    states: Vec<usize>,
    /// The offset that each command produces.
    offsets: Vec<isize>,
    /// The current position of the iterator.
    step: usize,
}

impl<'s> LabIterator<'s> {
    /// Create a new iterator.
    fn new(lab: &'s Lab) -> Self {
        let command_iterators: Vec<_> = lab.commands.iter().map(Command::iter).collect();
        let states = command_iterators.iter().map(|i| i.len()).collect();
        let offsets = vec![0; lab.commands.len()];
        Self {
            lab,
            command_iterators,
            states,
            offsets,
            step: 0,
        }
    }
}

impl<'s> Iterator for LabIterator<'s> {
    type Item = Change;

    fn next(&mut self) -> Option<Self::Item> {
        if self.step >= self.states.iter().sum() {
            return None;
        }

        let mut available_steps = self.step;

        for (i, mut command_iter) in self.command_iterators.iter().copied().enumerate() {
            println!(
                "available: {}, states {}, command {:?}",
                available_steps, self.states[i], self.lab.commands[i].kind
            );

            if available_steps >= self.states[i] {
                available_steps -= self.states[i];
                continue;
            }

            let mut change = command_iter.nth(available_steps).unwrap();
            *self.offsets.get_mut(i).unwrap() += change.len();
            change.map_range(|x| {
                (x as isize + self.offsets.iter().take(i).sum::<isize>()) as usize
            });

            self.step += 1;
            return Some(change);
        }

        unreachable!()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let res = self.states.iter().sum();
        (res, Some(res))
    }
}

impl<'s> ExactSizeIterator for LabIterator<'s> {}

impl<'s> IntoIterator for &'s Lab {
    type Item = Change;
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
