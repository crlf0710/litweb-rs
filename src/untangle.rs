pub(crate) fn generate_output(
    mut writer: impl io::Write,
    source: Vec<SourceToplevelBlock>,
) -> Result<(), GenerationError> {
    let mut first_block = true;
    for source_line in source {
        match source_line {
            SourceToplevelBlock::Preamble { lang } => {
                assert!(matches!(lang, SourceLanguage::Djot));
            }
            SourceToplevelBlock::VerbatimBlock { lang, lines } => {
                if !mem::replace(&mut first_block, false) {
                    writeln!(writer)?;
                }
                assert!(matches!(lang, GeneratedLanguage::Rust));
                let (fence_start, fence_end) = calc_djot_fences(lang, &lines);
                writeln!(writer, "{}", fence_start)?;
                for line in lines {
                    writeln!(writer, "{}", line)?;
                }
                writeln!(writer, "{}", fence_end)?;
            }
            SourceToplevelBlock::LiterateBlock { lines } => {
                if !mem::replace(&mut first_block, false) {
                    writeln!(writer)?;
                }
                for line in lines {
                    writeln!(writer, "{}", line)?;
                }
            }
            SourceToplevelBlock::Postamble => {}
        }
    }
    Ok(())
}

fn calc_djot_fences(lang: GeneratedLanguage, lines: &[String]) -> (String, String) {
    let mut fence_len = 3;
    for line in lines.iter() {
        let line_len = line.len();
        if line_len < fence_len {
            continue;
        }
        if line.chars().all(|ch| ch == '`') {
            fence_len = line_len + 1;
        }
    }
    let fence_end = std::iter::repeat('`').take(fence_len).collect();
    let fence_start = format!(
        "{fence_end} {}",
        match lang {
            GeneratedLanguage::Rust => "rust",
        }
    );
    (fence_start, fence_end)
}

pub(crate) fn convert_line_groups(
    generated: Vec<GeneratedLineGroup>,
    generated_lang: GeneratedLanguage,
) -> Result<Vec<SourceToplevelBlock>, ConversionError> {
    let mut generated: VecDeque<_> = generated.into();
    let Some(GeneratedLineGroup::Preamble) = generated.pop_front() else {
        return Err(ConversionError::NoValidPreamble);
    };
    let Some(GeneratedLineGroup::Postamble { source_lang }) = generated.pop_back() else {
        return Err(ConversionError::NoValidPostamble);
    };
    assert!(matches!(source_lang, SourceLanguage::Djot));
    let mut result_deque = VecDeque::new();
    result_deque.push_back(SourceToplevelBlock::Preamble {
        lang: SourceLanguage::Djot,
    });
    for line in generated {
        match line {
            GeneratedLineGroup::CodeLineGroup(lines) => {
                result_deque.push_back(SourceToplevelBlock::VerbatimBlock {
                    lang: generated_lang,
                    lines,
                })
            }
            GeneratedLineGroup::Preamble | GeneratedLineGroup::Postamble { .. } => {
                return Err(ConversionError::UnexpectedGeneratedLineGroup);
            }
        }
    }
    result_deque.push_back(SourceToplevelBlock::Postamble);
    Ok(result_deque.into())
}

pub(crate) fn analyze_line_groups(
    reader: impl io::Read,
) -> Result<Vec<GeneratedLineGroup>, AnalysisError> {
    let reader = BufReader::new(reader);
    let mut lines = reader.lines().collect::<Result<Vec<_>, _>>()?;
    let mut result_deque = VecDeque::new();
    let Some(ref postamble @ GeneratedLineGroup::Postamble { source_lang }) = take_postamble(&mut lines) else {
        return Err(AnalysisError::NoValidPostamble);
    };
    assert!(matches!(source_lang, SourceLanguage::Djot));
    result_deque.push_back(GeneratedLineGroup::Preamble);
    result_deque.push_back(GeneratedLineGroup::CodeLineGroup(lines));
    result_deque.push_back(postamble.clone());

    Ok(result_deque.into())
}

fn take_postamble(lines: &mut Vec<String>) -> Option<GeneratedLineGroup> {
    drop_trailing_empty_lines(lines);
    let last_line = lines.last()?;
    if last_line.trim() != GENERATED_SIGNATURE {
        return None;
    }
    lines.pop();
    Some(GeneratedLineGroup::Postamble {
        source_lang: SourceLanguage::Djot,
    })
}

fn drop_trailing_empty_lines(lines: &mut Vec<String>) {
    while let Some(last_line) = lines.last() {
        if !last_line.is_empty() {
            return;
        }
        lines.pop();
    }
}

use crate::tangle_and_untangle::GeneratedLanguage;
use crate::tangle_and_untangle::SourceLanguage;
use crate::tangle_and_untangle::GENERATED_SIGNATURE;

use crate::tangle_and_untangle::GeneratedLineGroup;
use crate::tangle_and_untangle::SourceToplevelBlock;

use crate::tangle_and_untangle::AnalysisError;
use crate::tangle_and_untangle::ConversionError;
use crate::tangle_and_untangle::GenerationError;

use std::{
    collections::VecDeque,
    io::{self, BufRead, BufReader},
    mem,
};
