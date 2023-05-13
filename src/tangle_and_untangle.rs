#[derive(Clone)]
pub(crate) enum SourceToplevelBlock {
    Preamble {
        lang: SourceLanguage,
    },
    VerbatimBlock {
        lang: GeneratedLanguage,
        lines: Vec<String>,
    },
    LiterateBlock {
        lines: Vec<String>,
    },
    Postamble,
}

#[derive(Clone)]
pub(crate) enum GeneratedLineGroup {
    Preamble,
    CodeLineGroup(Vec<String>),
    Postamble { source_lang: SourceLanguage },
}

#[derive(Clone, Copy)]
pub(crate) enum GeneratedLanguage {
    Rust,
}

pub(crate) const GENERATED_SIGNATURE: &'static str = "// [LITWEB djot->rust]";

#[derive(Clone, Copy)]
pub(crate) enum SourceLanguage {
    Djot,
}

#[derive(Error, Debug)]
pub enum GenerationError {
    #[error(transparent)]
    Io(#[from] io::Error),
}

#[derive(Error, Debug)]
pub enum ConversionError {
    #[error("No valid preamble is found")]
    NoValidPreamble,
    #[error("No valid postamble is found")]
    NoValidPostamble,
    #[error("Unexpected generated line group is met")]
    UnexpectedGeneratedLineGroup,
}

#[derive(Error, Debug)]
pub enum AnalysisError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("No valid postamble is found")]
    NoValidPostamble,
    #[error("Invalid Djot block event occurrred.")]
    InvalidDjotBlock,
}

use std::io;
use thiserror::Error;
