use std::{
    cmp::Reverse,
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
    process::{Command, Stdio},
};

use anyhow::{Context, Result, anyhow, bail};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};

use crate::config::LanguageConfig;

pub fn injectfmt_file(path: impl AsRef<Path>, cfg: &LanguageConfig, check: bool) -> Result<bool> {
    let path = path.as_ref();
    let mut file = File::options()
        .read(true)
        .write(!check)
        .open(path)
        .with_context(|| path.display().to_string())?;

    let mut src = String::new();
    file.read_to_string(&mut src)?;

    let new_src = injectfmt_str(&src, cfg)?;

    if !check && let Some(ref new_src) = new_src {
        file.set_len(0)?;
        file.seek(SeekFrom::Start(0))?;
        file.write_all(new_src.as_bytes())?;
        file.flush()?;
    }

    Ok(new_src.is_some())
}

pub fn injectfmt_str(src: &str, cfg: &LanguageConfig) -> Result<Option<String>> {
    let mut parser = Parser::new();
    parser.set_language(&cfg.language.into())?;

    let tree = parser
        .parse(src, None)
        .expect("The parser has had a language assigned with `Parser::set_language`");
    let root_node = tree.root_node();

    let query = Query::new(&tree.language(), &cfg.query)?;

    let mut cursor = QueryCursor::new();

    let mut matches = cursor
        .captures(&query, root_node, src.as_bytes())
        .filter_map(|(v, i)| {
            (query.capture_names()[*i] == "injectfmt").then(|| v.captures[*i].node)
        })
        .copied()
        .collect::<Vec<_>>();

    matches.sort_unstable_by_key(|v| Reverse(v.start_byte()));

    let outputs = matches
        .into_par_iter()
        .map(|v| {
            let input = &src[v.byte_range()];

            let mut child = Command::new(&cfg.format[0])
                .args(&cfg.format[1..])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()?;

            let mut stdin = child
                .stdin
                .take()
                .ok_or_else(|| anyhow!("failed to acquire STDIN from child process"))?;

            stdin.write_all(input.as_bytes())?;
            stdin.flush()?;

            drop(stdin);

            let output = child.wait_with_output()?;

            if !output.status.success() {
                if output.stderr.is_empty() {
                    bail!("Error from {}", &cfg.format[0]);
                } else {
                    let err = String::from_utf8(output.stderr)?;
                    bail!("Error from {}: {err}", &cfg.format[0]);
                }
            }

            let mut formatted = String::from_utf8(output.stdout)?;

            if let Some((i, _)) = input
                .char_indices()
                .rev()
                .take_while(|(_, v)| matches!(v, ' ' | '\t'))
                .last()
            {
                formatted.push_str(&input[i..]);
            }

            let input_start = v.start_byte();

            let start_idx = src[..input_start]
                .char_indices()
                .rev()
                .take_while(|(_, v)| matches!(v, ' ' | '\t'))
                .last()
                .map(|(i, _)| i)
                .unwrap_or(input_start);

            Ok(if formatted != input || start_idx != input_start {
                Some((start_idx..v.end_byte(), formatted))
            } else {
                None
            })
        })
        .collect::<Vec<_>>();

    let mut new_src = None;

    for v in outputs {
        if let Some((range, formatted)) = v? {
            new_src
                .get_or_insert_with(|| src.to_string())
                .replace_range(range, &formatted);
        }
    }

    Ok(new_src)
}
