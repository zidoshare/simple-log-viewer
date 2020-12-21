use rustyline::completion::{Completer, FilenameCompleter};
pub struct RLHelper {
    completer: FilenameCompleter,
    highligter: LineHighligter,
}

struct LineHighligter {}
