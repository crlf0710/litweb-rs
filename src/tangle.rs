pub(crate) fn generate_output(
    mut writer: impl io::Write,
    generated: Vec<GeneratedLineGroup>,
) -> Result<(), GenerationError> {
    let mut first_group = true;
    for line_group in generated {
        match line_group {
            GeneratedLineGroup::Preamble => {}
            GeneratedLineGroup::CodeLineGroup(lines) => {
                if !mem::replace(&mut first_group, false) {
                    writeln!(writer)?;
                }
                for line in lines {
                    writeln!(writer, "{line}")?;
                }
            }
            GeneratedLineGroup::Postamble { source_lang } => {
                assert!(matches!(source_lang, SourceLanguage::Djot));
                if !mem::replace(&mut first_group, false) {
                    writeln!(writer)?;
                }
                writeln!(writer, "{signature}", signature = GENERATED_SIGNATURE)?;
            }
        }
    }
    Ok(())
}

pub(crate) fn convert_source_blocks(
    blocks: Vec<SourceToplevelBlock>,
) -> Result<Vec<GeneratedLineGroup>, ConversionError> {
    let mut generated = vec![];
    for block in blocks {
        match block {
            SourceToplevelBlock::Preamble { lang } => {
                assert!(matches!(lang, SourceLanguage::Djot));
                generated.push(GeneratedLineGroup::Preamble);
            }
            SourceToplevelBlock::VerbatimBlock { lang, lines } => {
                assert!(matches!(lang, GeneratedLanguage::Rust));
                generated.push(GeneratedLineGroup::CodeLineGroup(lines));
            }
            SourceToplevelBlock::LiterateBlock { lines } => {
                generated.push(GeneratedLineGroup::CodeLineGroup(
                    lines
                        .into_iter()
                        .map(|markup| format!("// {markup}"))
                        .collect(),
                ));
            }
            SourceToplevelBlock::Postamble => {
                generated.push(GeneratedLineGroup::Postamble {
                    source_lang: SourceLanguage::Djot,
                });
            }
        }
    }
    Ok(generated)
}

pub(crate) fn analyze_source_blocks(
    mut reader: impl io::Read,
) -> Result<Vec<SourceToplevelBlock>, AnalysisError> {
    let mut source = String::default();
    let _ = reader.read_to_string(&mut source)?;
    let source = &source[..];
    let mut parser = DjotParser::new(source).into_offset_iter();
    let mut source_line_groups = vec![];
    source_line_groups.push(SourceToplevelBlock::Preamble {
        lang: SourceLanguage::Djot,
    });
    let top_level_block_iter =
        std::iter::from_fn(|| pull_next_top_level_block(&mut parser)).enumerate();
    for (_idx, (events, range)) in top_level_block_iter {
        match events.get(0) {
            Some(DjotEvent::Start(DjotContainer::CodeBlock { language }, ..))
                if *language == "rust" =>
            {
                match events.last() {
                    Some(DjotEvent::End(DjotContainer::CodeBlock { language }))
                        if *language == "rust" => {}
                    _ => return Err(AnalysisError::InvalidDjotBlock),
                }
                let events_count = events.len();
                let inner_events_range = 1..events_count - 1;
                let mut lines = vec![];
                for event in &events[inner_events_range] {
                    match event {
                        DjotEvent::Str(s) => {
                            lines.push(s.trim_end().to_string());
                        }
                        _ => unreachable!(),
                    }
                }
                source_line_groups.push(SourceToplevelBlock::VerbatimBlock {
                    lang: GeneratedLanguage::Rust,
                    lines: lines,
                });
            }
            _ => {
                let block_source = &source[range];
                source_line_groups.push(SourceToplevelBlock::LiterateBlock {
                    lines: block_source
                        .lines()
                        .map(|s| s.trim_end().to_string())
                        .collect(),
                });
            }
        }
    }
    source_line_groups.push(SourceToplevelBlock::Postamble);
    Ok(source_line_groups)
}

fn pull_next_top_level_block<'input>(
    parser: &mut DjotParserWithOffset<'input>,
) -> Option<(Vec<DjotEvent<'input>>, Range<usize>)> {
    let mut parser = parser.peekable();
    let (first_token, first_token_range) = parser.next()?;
    match &first_token {
        _ if is_singleton_block(&first_token) => Some((vec![first_token], first_token_range)),
        DjotEvent::Start(start_container, _) if is_block_container(start_container) => {
            let mut nesting_counter = 1usize;
            let mut tokens = vec![first_token.clone()];
            let mut total_range = first_token_range;
            while let Some((next_token, next_token_range)) = parser.next() {
                match &next_token {
                    DjotEvent::Start(next_container, _) if start_container == next_container => {
                        nesting_counter += 1;
                    }
                    DjotEvent::End(next_container) if start_container == next_container => {
                        nesting_counter -= 1;
                    }
                    _ => {}
                }
                tokens.push(next_token);
                total_range = union_range(total_range, next_token_range);
                if nesting_counter == 0 {
                    break;
                }
            }
            Some((tokens, total_range))
        }
        _ => None,
    }
}

fn is_singleton_block(event: &DjotEvent) -> bool {
    match event {
        DjotEvent::Blankline | DjotEvent::ThematicBreak(_) => true,
        DjotEvent::Start(_, _)
        | DjotEvent::End(_)
        | DjotEvent::Str(_)
        | DjotEvent::FootnoteReference(_)
        | DjotEvent::Symbol(_)
        | DjotEvent::LeftSingleQuote
        | DjotEvent::RightSingleQuote
        | DjotEvent::LeftDoubleQuote
        | DjotEvent::RightDoubleQuote
        | DjotEvent::Ellipsis
        | DjotEvent::EnDash
        | DjotEvent::EmDash
        | DjotEvent::NonBreakingSpace
        | DjotEvent::Softbreak
        | DjotEvent::Hardbreak
        | DjotEvent::Escape => false,
    }
}

fn is_block_container(container: &DjotContainer) -> bool {
    match container {
        DjotContainer::Blockquote
        | DjotContainer::List { .. }
        | DjotContainer::ListItem
        | DjotContainer::TaskListItem { .. }
        | DjotContainer::DescriptionList
        | DjotContainer::DescriptionDetails
        | DjotContainer::Footnote { .. }
        | DjotContainer::Table
        | DjotContainer::TableRow { .. }
        | DjotContainer::Section { .. }
        | DjotContainer::Div { .. }
        | DjotContainer::Paragraph
        | DjotContainer::Heading { .. }
        | DjotContainer::TableCell { .. }
        | DjotContainer::Caption
        | DjotContainer::DescriptionTerm
        | DjotContainer::LinkDefinition { .. }
        | DjotContainer::RawBlock { .. }
        | DjotContainer::CodeBlock { .. } => true,
        DjotContainer::Span
        | DjotContainer::Link(_, _)
        | DjotContainer::Image(_, _)
        | DjotContainer::Verbatim
        | DjotContainer::Math { .. }
        | DjotContainer::RawInline { .. }
        | DjotContainer::Subscript
        | DjotContainer::Superscript
        | DjotContainer::Insert
        | DjotContainer::Delete
        | DjotContainer::Strong
        | DjotContainer::Emphasis
        | DjotContainer::Mark => false,
    }
}

fn union_range(lhs: Range<usize>, rhs: Range<usize>) -> Range<usize> {
    let start = Ord::min(lhs.start, rhs.start);
    let end = Ord::max(lhs.end, rhs.end);
    start..end
}

use crate::tangle_and_untangle::GeneratedLanguage;
use crate::tangle_and_untangle::GeneratedLineGroup;
use crate::tangle_and_untangle::SourceLanguage;
use crate::tangle_and_untangle::SourceToplevelBlock;
use crate::tangle_and_untangle::GENERATED_SIGNATURE;

use crate::tangle_and_untangle::AnalysisError;
use crate::tangle_and_untangle::ConversionError;
use crate::tangle_and_untangle::GenerationError;

use std::io;
use std::mem;
use std::ops::Range;

use jotdown::Container as DjotContainer;
use jotdown::Event as DjotEvent;
use jotdown::OffsetIter as DjotParserWithOffset;
use jotdown::Parser as DjotParser;
