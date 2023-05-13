use std::{fs::File, io, path::Path};
use thiserror::Error;

enum FileType {
    SourceDjot,
    GeneratedMarkdown,
    GeneratedRustModule,
}

#[macro_use]
mod utils {
    use crate::{FileType, TangleOrWeave};
    use std::{
        fs, io,
        path::{Path, PathBuf},
        time::SystemTime,
    };

    trait ExtensionEq {
        fn extension_eq(&self, v: &str) -> bool;
    }

    impl ExtensionEq for Path {
        fn extension_eq(&self, rhs: &str) -> bool {
            match self.extension() {
                Some(lhs) => lhs == rhs,
                None => rhs.is_empty(),
            }
        }
    }

    pub(crate) fn determine_filetype_and_dest(
        input: &Path,
        mode: TangleOrWeave,
    ) -> Option<(FileType, PathBuf, FileType)> {
        if input.extension_eq("djot") && input.with_extension("").extension_eq("lit") {
            let (dest_ext, dest_type) = match mode {
                TangleOrWeave::Tangle => ("rs", FileType::GeneratedRustModule),
                TangleOrWeave::Weave => ("md", FileType::GeneratedMarkdown),
            };
            let dest = input.with_extension("").with_extension(dest_ext);
            Some((FileType::SourceDjot, dest, dest_type))
        } else if matches!(mode, TangleOrWeave::Tangle) && input.extension_eq("rs") {
            Some((
                FileType::GeneratedRustModule,
                input.with_extension("lit.djot"),
                FileType::SourceDjot,
            ))
        } else if matches!(mode, TangleOrWeave::Weave) && input.extension_eq("md") {
            Some((
                FileType::GeneratedMarkdown,
                input.with_extension("lit.djot"),
                FileType::SourceDjot,
            ))
        } else {
            None
        }
    }

    pub(crate) fn ensure_input_is_newer(
        input: &Path,
        output: &Path,
        force: bool,
    ) -> Result<Option<SystemTime>, io::Error> {
        let input_modified = fs::metadata(input)?.modified()?;
        let Some(output_modified) = fs::metadata(output).ok().and_then(|m| m.modified().ok()) else {
            return Ok(Some(input_modified));
        };
        if input_modified > output_modified || force {
            return Ok(Some(input_modified));
        }
        Ok(None)
    }

    /*
    macro_rules! impl_from_ty_for_ty {
        ($from_ty:path, $self_ty:path, $ctor:expr) => {
            impl From<$from_ty> for $self_ty {
                fn from(value: $from_ty) -> Self {
                    ($ctor)(value)
                }
            }
        };
    }
     */
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
enum TangleOrWeave {
    Tangle,
    Weave,
}

mod tangle_and_untangle;

mod tangle;
mod untangle;

#[derive(Error, Debug)]
pub enum TangleUntangleError {
    #[error("File extension is unrecognized")]
    UnrecognizedFileExt,
    #[error("Input file is not newer")]
    InputFileIsNotNewer,
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    TangleUntangleAnalysisError(#[from] tangle_and_untangle::AnalysisError),
    #[error(transparent)]
    TangleUntangleConversionError(#[from] tangle_and_untangle::ConversionError),
    #[error(transparent)]
    TangleUntangleGenerationError(#[from] tangle_and_untangle::GenerationError),
}

pub fn tangle_or_untangle(
    file_path: &Path,
    output_path: Option<&Path>,
    force: bool,
) -> Result<(), TangleUntangleError> {
    let Some((file_type, default_output_path, output_file_type)) = utils::determine_filetype_and_dest(file_path, TangleOrWeave::Tangle) else {
        return Err(TangleUntangleError::UnrecognizedFileExt);
    };
    let output_path = match output_path {
        None => default_output_path,
        Some(path) => path.to_owned(),
    };
    let is_tangle = matches!(file_type, FileType::SourceDjot);
    let Some(output_time_to_use) = utils::ensure_input_is_newer(file_path, &output_path, force)? else {
        return Err(TangleUntangleError::InputFileIsNotNewer);
    };
    let input_file = File::open(file_path)?;
    let output_file;
    if !is_tangle {
        assert!(matches!(output_file_type, FileType::SourceDjot));
        let generated_lines = untangle::analyze_line_groups(&input_file)?;
        let source_lines = untangle::convert_line_groups(
            generated_lines,
            tangle_and_untangle::GeneratedLanguage::Rust,
        )?;
        output_file = File::create(&output_path)?;
        untangle::generate_output(&output_file, source_lines)?;
    } else {
        assert!(matches!(output_file_type, FileType::GeneratedRustModule));
        let source_blocks = tangle::analyze_source_blocks(&input_file)?;
        let generated_lines = tangle::convert_source_blocks(source_blocks)?;
        output_file = File::create(&output_path)?;
        tangle::generate_output(&output_file, generated_lines)?;
    }
    drop(output_file);
    filetime::set_file_mtime(
        &output_path,
        filetime::FileTime::from_system_time(output_time_to_use),
    )?;
    Ok(())
}

mod weave {}
mod unweave {}
