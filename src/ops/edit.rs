//! dotman.yaml editing for `dotman new-link`.
//!
//! Edits are surgical line-level insertions: only the touched `links` lines
//! change, so comments and formatting anywhere else in the file (including
//! inside the links section) survive byte-for-byte. The edited document is
//! re-parsed and compared against the original before it is accepted; on any
//! ambiguity the call fails without the caller writing anything to disk.

use serde_yaml::Value;

/// Add `target -> source` to the `links` section of a dotman.yaml document.
///
/// Supports the block map form (`~/.config/fish: config/fish`) and the block
/// list form (`- target: ...` / `source: ...`). List entries with extra
/// fields such as `backup`/`relink` are preserved as-is. If `target` already
/// exists, only its source is updated in place (a trailing comment on the
/// line is kept). A missing `links` section gets one appended at the end.
pub fn add_link_entry(raw: &str, target: &str, source: &str) -> Result<String, String> {
    if target.contains('\n') || source.contains('\n') {
        return Err("target and source must be single-line values".into());
    }
    let doc: Value =
        serde_yaml::from_str(raw).map_err(|e| format!("failed to parse dotman.yaml: {e}"))?;
    if !doc.is_mapping() {
        return Err("dotman.yaml is not a mapping".into());
    }

    let form = LinksForm::from_doc(&doc)?;
    let old_pairs = links_pairs(&doc)?;
    let target_count = old_pairs.iter().filter(|(t, _)| t == target).count();
    if target_count > 1 {
        return Err(format!(
            "link '{target}' appears multiple times in the links section; \
             edit dotman.yaml manually"
        ));
    }
    let target_repr = scalar_repr(target)?;
    let source_repr = scalar_repr(source)?;

    let edited = if matches!(form, LinksForm::Missing) {
        append_section(raw, &target_repr, &source_repr)
    } else if target_count == 1 {
        replace_entry(raw, &form, target, &source_repr)
    } else {
        append_entry(raw, &form, &target_repr, &source_repr)
    }?;

    verify_edit(raw, &edited, target, source)?;
    Ok(edited)
}

/// How the `links` section is represented in the document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LinksForm {
    /// No `links` key at all.
    Missing,
    /// `links` with no entries, mapping-shaped (bare `links:`, `links: {}`, null).
    EmptyMap,
    /// `links: []` — no entries, sequence-shaped.
    EmptySeq,
    /// Block mapping form (`~/.config/fish: config/fish`).
    Map,
    /// Block sequence form (`- target: ...` items).
    Seq,
}

impl LinksForm {
    fn from_doc(doc: &Value) -> Result<Self, String> {
        match doc.get("links") {
            None => Ok(Self::Missing),
            Some(Value::Null) => Ok(Self::EmptyMap),
            Some(Value::Mapping(m)) if m.is_empty() => Ok(Self::EmptyMap),
            Some(Value::Sequence(s)) if s.is_empty() => Ok(Self::EmptySeq),
            Some(Value::Mapping(_)) => Ok(Self::Map),
            Some(Value::Sequence(_)) => Ok(Self::Seq),
            Some(_) => {
                Err("links: section has an unexpected format; edit dotman.yaml manually".into())
            }
        }
    }
}

/// Extract (target, source) pairs from the parsed `links` value.
///
/// Sequence items may carry extra fields (`backup`, `relink`); they are
/// ignored here but never rejected.
fn links_pairs(doc: &Value) -> Result<Vec<(String, String)>, String> {
    let Some(links) = doc.get("links") else {
        return Ok(Vec::new());
    };
    match links {
        Value::Null => Ok(Vec::new()),
        Value::Mapping(m) => m
            .iter()
            .map(|(key, value)| {
                let key = key
                    .as_str()
                    .ok_or_else(|| "links: mapping keys must be strings".to_string())?;
                let value = value
                    .as_str()
                    .ok_or_else(|| "links: mapping values must be strings".to_string())?;
                Ok((key.to_string(), value.to_string()))
            })
            .collect(),
        Value::Sequence(items) => items
            .iter()
            .map(|item| {
                let item_map = item
                    .as_mapping()
                    .ok_or_else(|| "links: list item is not a mapping".to_string())?;
                let target = item_map
                    .get("target")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "links: list item missing a 'target' string".to_string())?;
                let source = item_map
                    .get("source")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "links: list item missing a 'source' string".to_string())?;
                Ok((target.to_string(), source.to_string()))
            })
            .collect(),
        _ => Err("links: section has an unexpected format; edit dotman.yaml manually".into()),
    }
}

/// Serialize a single scalar the way serde_yaml would write it (quoting only
/// when the plain form would be ambiguous).
fn scalar_repr(s: &str) -> Result<String, String> {
    serde_yaml::to_string(&Value::String(s.to_string()))
        .map(|repr| repr.trim_end().to_string())
        .map_err(|e| format!("failed to serialize YAML value: {e}"))
}

// ---- Verification ----

/// Re-parse the edited document and require semantic equality with the
/// original plus the requested change. Any mismatch fails without writing.
fn verify_edit(raw: &str, edited: &str, target: &str, source: &str) -> Result<(), String> {
    let old_doc: Value =
        serde_yaml::from_str(raw).map_err(|e| format!("failed to parse dotman.yaml: {e}"))?;
    let new_doc: Value = serde_yaml::from_str(edited)
        .map_err(|e| format!("edit produced invalid YAML ({e}); file left unchanged"))?;
    let old_pairs = links_pairs(&old_doc)?;
    let new_pairs = links_pairs(&new_doc)?;

    let mut expected: Vec<(String, String)> = old_pairs
        .iter()
        .filter(|(t, _)| t != target)
        .cloned()
        .collect();
    expected.push((target.to_string(), source.to_string()));
    expected.sort();

    let mut actual = new_pairs;
    actual.sort();
    if actual != expected {
        return Err(
            "internal error: edited document does not match the requested change; \
             file left unchanged"
                .into(),
        );
    }
    Ok(())
}

// ---- Line-level editing ----

/// Append a whole new `links` section when the document has none, matching
/// the file's existing line endings so the result has no mixed EOLs.
fn append_section(raw: &str, target_repr: &str, source_repr: &str) -> Result<String, String> {
    let sep = if raw.contains("\r\n") { "\r\n" } else { "\n" };
    let mut out = raw.to_string();
    if !out.ends_with('\n') {
        out.push_str(sep);
    }
    out.push_str(&format!("links:{sep}  {target_repr}: {source_repr}{sep}"));
    Ok(out)
}

/// Atomically replace the contents of `path`, preserving symlinks and the
/// original file permissions.
///
/// The symlink itself is kept (its target is what gets replaced): the real
/// file is resolved first, a unique temporary file is created next to it with
/// `create_new`, fully written and synced with the original permissions, and
/// `rename` moves it into place over the real file. Any failure before the
/// rename removes the temporary file best-effort and returns context.
pub fn write_config_atomic(path: &std::path::Path, contents: &str) -> Result<(), String> {
    use std::fs::OpenOptions;
    use std::io::Write;

    let real = std::fs::canonicalize(path)
        .map_err(|e| format!("failed to resolve {}: {e}", path.display()))?;
    let parent = real
        .parent()
        .ok_or_else(|| format!("cannot determine parent directory of {}", real.display()))?;
    let file_name = real
        .file_name()
        .ok_or_else(|| format!("cannot determine file name of {}", real.display()))?
        .to_string_lossy()
        .into_owned();
    let permissions = std::fs::metadata(&real).ok().map(|m| m.permissions());

    // Unique temporary file in the same directory (create_new).
    let mut n = 0u32;
    let (tmp_path, mut tmp) = loop {
        let name = format!(".{file_name}.tmp.{}.{n}", std::process::id());
        let candidate = parent.join(&name);
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => break (candidate, file),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => n += 1,
            Err(e) => return Err(format!("failed to create temporary file: {e}")),
        }
    };

    let result = (|| -> std::io::Result<()> {
        tmp.write_all(contents.as_bytes())?;
        if let Some(perms) = &permissions {
            tmp.set_permissions(perms.clone())?;
        }
        tmp.sync_all()?;
        drop(tmp);
        std::fs::rename(&tmp_path, &real)?;
        Ok(())
    })();

    if let Err(e) = result {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(format!("failed to write {}: {e}", path.display()));
    }
    Ok(())
}

/// Append a new entry/item to an existing `links` section.
fn append_entry(
    raw: &str,
    form: &LinksForm,
    target_repr: &str,
    source_repr: &str,
) -> Result<String, String> {
    let mut lines: Vec<String> = raw.split('\n').map(str::to_string).collect();
    let (key_idx, key_indent) = locate_links_key(&lines)?;

    if matches!(form, LinksForm::EmptyMap | LinksForm::EmptySeq) {
        rewrite_empty_links_line(&mut lines, key_idx)?;
        // Land the new entry below any leading comment that documents the
        // (previously empty) section rather than above it.
        let insert_at = after_leading_comments(&lines, key_idx);
        let (target_line, eol) = new_entry_lines(form, key_indent + 2, target_repr, source_repr);
        lines.insert(insert_at, target_line);
        if let Some(eol) = eol {
            lines.insert(insert_at + 1, eol);
        }
        return Ok(rejoin(&lines, raw));
    }

    reject_inline_value(&lines, key_idx)?;
    // A block/folded scalar entry spans indented continuation lines, so
    // appending after it would corrupt the section's layout.
    if section_has_block_scalar(&lines, key_idx, key_indent) {
        return Err(
            "links contains a block/folded scalar (| or >) entry that cannot be safely \
             appended after; edit dotman.yaml manually"
                .into(),
        );
    }
    let content_idx = last_content_line(&lines, key_idx, key_indent);
    let entry_indent = entry_indent(&lines, key_idx, key_indent, key_indent + 2);
    let (target_line, eol) = new_entry_lines(form, entry_indent, target_repr, source_repr);
    lines.insert(content_idx + 1, target_line);
    if let Some(eol) = eol {
        lines.insert(content_idx + 2, eol);
    }
    Ok(rejoin(&lines, raw))
}

/// Index just past any blank/comment lines that directly follow the `links:`
/// key, so a new entry lands below the section's leading comment. Stops
/// before the file's trailing empty element (produced by a final newline),
/// which belongs at the end of the document.
fn after_leading_comments(lines: &[String], key_idx: usize) -> usize {
    let mut idx = key_idx + 1;
    while idx < lines.len() {
        let line = &lines[idx];
        if line.trim().is_empty() {
            if idx == lines.len() - 1 {
                break; // trailing element from the final \n
            }
            idx += 1;
        } else if line.trim_start().starts_with('#') {
            idx += 1;
        } else {
            break;
        }
    }
    idx
}

/// Whether any content line of the section is a block/folded scalar
/// indicator (`key: |`, `key: >`, ...).
fn section_has_block_scalar(lines: &[String], key_idx: usize, key_indent: usize) -> bool {
    section_content_lines(lines, key_idx, key_indent)
        .into_iter()
        .any(|idx| {
            let content = lines[idx].trim_start();
            split_key_value(content).is_some_and(|(_, rest)| {
                is_block_scalar_indicator(split_value_comment(rest).0.trim())
            })
        })
}

/// Update the source of an existing target in place, keeping the rest of the
/// line (including any trailing comment) intact.
fn replace_entry(
    raw: &str,
    form: &LinksForm,
    target: &str,
    source_repr: &str,
) -> Result<String, String> {
    let mut lines: Vec<String> = raw.split('\n').map(str::to_string).collect();
    let (key_idx, key_indent) = locate_links_key(&lines)?;

    let target_idx = match form {
        LinksForm::Map => find_map_entry_line(&lines, key_idx, key_indent, target),
        _ => find_list_item_line(&lines, key_idx, key_indent, target),
    }
    .ok_or_else(|| {
        format!(
            "link '{target}' already exists but its entry could not be located \
             (unusual quoting or formatting); edit dotman.yaml manually"
        )
    })?;

    let source_idx = match form {
        LinksForm::Map => target_idx,
        _ => find_list_item_source_line(&lines, target_idx).ok_or_else(|| {
            format!("link '{target}' exists but has no source line to update; edit manually")
        })?,
    };

    replace_source_on_line(&mut lines, source_idx, source_repr)?;
    Ok(rejoin(&lines, raw))
}

/// Locate the `links:` key line. Prefers the least-indented candidate and
/// errors when several equally-placed candidates exist.
fn locate_links_key(lines: &[String]) -> Result<(usize, usize), String> {
    let candidates: Vec<(usize, usize)> = lines
        .iter()
        .enumerate()
        .filter_map(|(idx, line)| is_links_key(line).map(|indent| (idx, indent)))
        .collect();
    let Some((_, min_indent)) = candidates.iter().min_by_key(|(_, indent)| *indent) else {
        return Err("cannot locate the links section; edit dotman.yaml manually".into());
    };
    let at_min: Vec<usize> = candidates
        .iter()
        .filter(|(_, indent)| *indent == *min_indent)
        .map(|(idx, _)| *idx)
        .collect();
    if at_min.len() != 1 {
        return Err("links section is ambiguous; edit dotman.yaml manually".into());
    }
    Ok((at_min[0], *min_indent))
}

/// Return the indentation of a line whose trimmed content is a `links:` key
/// (a `links:` token at the start, followed by optional spaces and a colon).
fn is_links_key(line: &str) -> Option<usize> {
    let indent = line.len() - line.trim_start().len();
    let trimmed = line.trim_start();
    let rest = trimmed.strip_prefix("links")?;
    let rest = rest.trim_start_matches([' ', '\t']);
    rest.starts_with(':').then_some(indent)
}

/// Content lines belonging to the links section: non-blank, non-comment lines
/// indented deeper than the key, stopping at the next top-level key.
fn section_content_lines(lines: &[String], key_idx: usize, key_indent: usize) -> Vec<usize> {
    let mut indices = Vec::new();
    for (idx, line) in lines.iter().enumerate().skip(key_idx + 1) {
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        if indent <= key_indent {
            break;
        }
        indices.push(idx);
    }
    indices
}

fn last_content_line(lines: &[String], key_idx: usize, key_indent: usize) -> usize {
    section_content_lines(lines, key_idx, key_indent)
        .last()
        .copied()
        .unwrap_or(key_idx)
}

/// Indent used by existing entries in the section, or `fallback` when the
/// section has no entries yet.
fn entry_indent(lines: &[String], key_idx: usize, key_indent: usize, fallback: usize) -> usize {
    section_content_lines(lines, key_idx, key_indent)
        .first()
        .map(|idx| lines[*idx].len() - lines[*idx].trim_start().len())
        .unwrap_or(fallback)
}

/// Build the line(s) for the new entry in the section's own form.
fn new_entry_lines(
    form: &LinksForm,
    indent: usize,
    target_repr: &str,
    source_repr: &str,
) -> (String, Option<String>) {
    let pad = " ".repeat(indent);
    match form {
        LinksForm::Seq | LinksForm::EmptySeq => (
            format!("{pad}- target: {target_repr}"),
            Some(format!("{pad}  source: {source_repr}")),
        ),
        _ => (format!("{pad}{target_repr}: {source_repr}"), None),
    }
}

/// Reject a non-empty inline value on the `links:` line (`links: {a: b}`),
/// which cannot be extended line-by-line without rewriting the section.
fn reject_inline_value(lines: &[String], key_idx: usize) -> Result<(), String> {
    let trimmed = lines[key_idx].trim_start();
    let Some(colon) = trimmed.find(':') else {
        return Ok(());
    };
    let (value, _) = split_value_comment(&trimmed[colon + 1..]);
    let value = value.trim();
    if !value.is_empty() {
        return Err(format!(
            "links has an inline value '{value}' that cannot be extended; \
             edit dotman.yaml manually"
        ));
    }
    Ok(())
}

/// Turn an inline empty `links: {}` / `links: []` / `links: null` line into a
/// bare `links:` key (preserving any trailing comment) so entries can follow.
fn rewrite_empty_links_line(lines: &mut [String], key_idx: usize) -> Result<(), String> {
    let line = &lines[key_idx];
    let trimmed = line.trim_start();
    let rest = &trimmed[trimmed
        .find(':')
        .ok_or_else(|| "links key line has no colon; edit dotman.yaml manually".to_string())?
        + 1..];
    let (value, comment) = split_value_comment(rest);
    let value = value.trim();
    if !value.is_empty() && !matches!(value, "{}" | "[]" | "null" | "Null" | "NULL" | "~") {
        return Err(format!(
            "links has an inline value '{value}' that cannot be extended; \
             edit dotman.yaml manually"
        ));
    }
    let comment = comment.trim_start();
    let indent = line.len() - line.trim_start().len();
    let mut new_line = format!("{}links:", " ".repeat(indent));
    if !comment.is_empty() {
        new_line.push(' ');
        new_line.push_str(comment);
    }
    lines[key_idx] = new_line;
    Ok(())
}

/// Find the map-form line whose key equals `target`, stripping surrounding
/// quotes for comparison.
fn find_map_entry_line(
    lines: &[String],
    key_idx: usize,
    key_indent: usize,
    target: &str,
) -> Option<usize> {
    section_content_lines(lines, key_idx, key_indent)
        .into_iter()
        .find(|&idx| map_entry_key(&lines[idx]).as_deref() == Some(target))
}

/// Find the sequence-form `- target: <target>` line.
fn find_list_item_line(
    lines: &[String],
    key_idx: usize,
    key_indent: usize,
    target: &str,
) -> Option<usize> {
    for idx in section_content_lines(lines, key_idx, key_indent) {
        let content = lines[idx].trim_start();
        let Some(rest) = content.strip_prefix("- target:") else {
            continue;
        };
        let (value, _) = split_value_comment(rest);
        if unquote(value.trim()) == Some(target) {
            return Some(idx);
        }
    }
    None
}

/// Within a list item starting at `target_idx`, find its `source:` line.
fn find_list_item_source_line(lines: &[String], target_idx: usize) -> Option<usize> {
    let item_indent = lines[target_idx].len() - lines[target_idx].trim_start().len();
    for (idx, line) in lines.iter().enumerate().skip(target_idx + 1) {
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        if indent <= item_indent {
            return None; // item ended without a source line
        }
        if map_entry_key(line).as_deref() == Some("source") {
            return Some(idx);
        }
    }
    None
}

/// Replace the value part of a `key: value` line with `source_repr`,
/// preserving any trailing comment.
fn replace_source_on_line(
    lines: &mut [String],
    idx: usize,
    source_repr: &str,
) -> Result<(), String> {
    let line = &lines[idx];
    let indent = line.len() - line.trim_start().len();
    let content = line.trim_start();
    let (key_part, rest) = split_key_value(content)
        .ok_or_else(|| "entry line has no colon; edit dotman.yaml manually".to_string())?;
    let (value, comment) = split_value_comment(rest);
    if is_block_scalar_indicator(value.trim()) {
        return Err(
            "link source uses a block/folded scalar (| or >), which cannot be updated \
             safely line-by-line; edit dotman.yaml manually"
                .into(),
        );
    }
    let comment = comment.trim_start();
    let mut new_line = format!("{}{}: {source_repr}", " ".repeat(indent), key_part);
    if !comment.is_empty() {
        new_line.push(' ');
        new_line.push_str(comment);
    }
    lines[idx] = new_line;
    Ok(())
}

/// Detect YAML block/folded scalar indicators (`|`, `>`, `|2`, `|-`, `>2-`,
/// ...). Such values span the following indented lines and cannot be replaced
/// safely with a single-line edit.
fn is_block_scalar_indicator(value: &str) -> bool {
    let mut chars = value.chars();
    match chars.next() {
        Some('|') | Some('>') => chars.all(|c| c.is_ascii_digit() || c == '+' || c == '-'),
        _ => false,
    }
}

/// Extract the key of a `key: value` content line, ignoring sequence items.
/// Surrounding quotes are stripped for comparison.
fn map_entry_key(line: &str) -> Option<String> {
    let content = line.trim_start();
    if content.starts_with("- ") {
        return None;
    }
    let (key, _) = split_key_value(content)?;
    let key = unquote(key.trim())?;
    if key.is_empty() {
        return None;
    }
    Some(key.to_string())
}

/// Split a `key: value` content line into the key text (as written) and the
/// remainder after the colon.
///
/// A plain key ends at the first colon followed by whitespace or end of line,
/// so `a:b: c` parses as key `a:b`. Quoted keys (`'a:b'`, `"a: b"`) are
/// scanned through their closing quote, honoring `''` and `\"` escapes.
fn split_key_value(content: &str) -> Option<(&str, &str)> {
    let bytes = content.as_bytes();
    let first = *bytes.first()?;
    if first == b'\'' || first == b'"' {
        let quote = first;
        let mut i = 1;
        while i < bytes.len() {
            match bytes[i] {
                b'\\' if quote == b'"' => i += 2, // escaped \" inside double quotes
                b'\'' if quote == b'\'' && i + 1 < bytes.len() && bytes[i + 1] == b'\'' => {
                    i += 2; // escaped '' inside single quotes
                }
                b if b == quote => {
                    let rest = content[i + 1..].trim_start();
                    let rest = rest.strip_prefix(':')?;
                    return Some((&content[..=i], rest));
                }
                _ => i += 1,
            }
        }
        return None; // unterminated quote
    }
    for (i, &b) in bytes.iter().enumerate() {
        if b == b':' && (i + 1 == bytes.len() || bytes[i + 1] == b' ' || bytes[i + 1] == b'\t') {
            return Some((&content[..i], &content[i + 1..]));
        }
    }
    None
}

fn unquote(s: &str) -> Option<&str> {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() >= 2 {
        let quote = chars[0];
        if (quote == '"' || quote == '\'') && chars[chars.len() - 1] == quote {
            return Some(&s[quote.len_utf8()..s.len() - quote.len_utf8()]);
        }
    }
    Some(s)
}

/// Split `key: value # comment` regions into (value, comment), honoring
/// single- and double-quoted strings so `#` inside quotes is not treated as
/// a comment start. YAML escapes (`''` in single quotes, `\"` in double
/// quotes) are skipped as pairs. A trailing `\r` from CRLF files is trimmed
/// so value comparisons stay clean; `rejoin` restores the line ending.
fn split_value_comment(region: &str) -> (&str, &str) {
    let region = region.trim_end_matches('\r');
    let chars: Vec<(usize, char)> = region.char_indices().collect();
    let mut in_single = false;
    let mut in_double = false;
    let mut prev_was_space = true;
    let mut i = 0;
    while i < chars.len() {
        let (idx, ch) = chars[i];
        match ch {
            '\'' if !in_double => {
                if in_single && i + 1 < chars.len() && chars[i + 1].1 == '\'' {
                    i += 2; // escaped '' inside single quotes
                    prev_was_space = false;
                    continue;
                }
                in_single = !in_single;
                prev_was_space = false;
            }
            '"' if !in_single => {
                if in_double && i + 1 < chars.len() && chars[i + 1].1 == '\\' {
                    i += 2; // escaped \" inside double quotes
                    prev_was_space = false;
                    continue;
                }
                in_double = !in_double;
                prev_was_space = false;
            }
            '#' if !in_single && !in_double && prev_was_space => {
                return (&region[..idx], &region[idx..]);
            }
            _ => {
                prev_was_space = ch.is_whitespace();
            }
        }
        i += 1;
    }
    (region, "")
}

fn rejoin(lines: &[String], raw: &str) -> String {
    let crlf = raw.contains("\r\n");
    let mut out = lines.join("\n");
    if crlf {
        // Restore CRLF line endings on lines that lack a `\r` (newly inserted
        // and replaced lines are built without one), avoiding mixed endings.
        let fixed: Vec<String> = out
            .split('\n')
            .map(|line| {
                if line.is_empty() || line.ends_with('\r') {
                    line.to_string()
                } else {
                    format!("{line}\r")
                }
            })
            .collect();
        out = fixed.join("\n");
    }
    if raw.ends_with('\n') && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn links_value(doc: &Value) -> &Value {
        doc.get("links").unwrap()
    }

    #[test]
    fn map_form_append_preserves_comments_and_layout() {
        let raw = "# dotman config\nlinks:\n  # 1. Terminal\n  ~/.config/fish: config/fish\n  # 2. Shell\ncreate: []\n";
        let out = add_link_entry(raw, "~/.tmux.conf", "config/tmux.conf").unwrap();
        // The new entry lands after the last entry line, before the section's
        // trailing comment; every other byte of the document is untouched.
        assert_eq!(
            out,
            "# dotman config\nlinks:\n  # 1. Terminal\n  ~/.config/fish: config/fish\n  ~/.tmux.conf: config/tmux.conf\n  # 2. Shell\ncreate: []\n"
        );
        assert_eq!(out.lines().next().unwrap(), "# dotman config");
        assert!(out.ends_with("create: []\n"));
    }

    #[test]
    fn map_form_replaces_source_keeping_trailing_comment() {
        let raw = "links:\n  ~/.config/fish: config/fish # main shell\n";
        let out = add_link_entry(raw, "~/.config/fish", "config/fish-new").unwrap();
        assert_eq!(
            out,
            "links:\n  ~/.config/fish: config/fish-new # main shell\n"
        );
    }

    #[test]
    fn sequence_form_appends_item_and_preserves_extra_fields() {
        let raw = "install: []\nlinks:\n  - target: ~/.config/fish\n    source: config/fish\n    backup: true\n";
        let out = add_link_entry(raw, "~/.tmux.conf", "config/tmux.conf").unwrap();
        assert_eq!(
            out,
            "install: []\nlinks:\n  - target: ~/.config/fish\n    source: config/fish\n    backup: true\n  - target: ~/.tmux.conf\n    source: config/tmux.conf\n"
        );
        let doc: Value = serde_yaml::from_str(&out).unwrap();
        let items = links_value(&doc).as_sequence().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["target"].as_str(), Some("~/.config/fish"));
        assert_eq!(items[0]["backup"].as_bool(), Some(true));
        assert_eq!(items[1]["target"].as_str(), Some("~/.tmux.conf"));
        assert_eq!(items[1]["source"].as_str(), Some("config/tmux.conf"));
    }

    #[test]
    fn sequence_form_updates_existing_target_source() {
        let raw = "links:\n  - target: ~/.config/fish\n    source: config/fish\n    backup: true\n";
        let out = add_link_entry(raw, "~/.config/fish", "config/fish2").unwrap();
        assert_eq!(
            out,
            "links:\n  - target: ~/.config/fish\n    source: config/fish2\n    backup: true\n"
        );
    }

    #[test]
    fn update_unquoted_key_containing_colon() {
        // `a:b: c` must be parsed as key `a:b`, so a link added earlier with a
        // colon in its target can be updated again.
        let raw = "links:\n  a:b: c\n";
        let out = add_link_entry(raw, "a:b", "d").unwrap();
        assert_eq!(out, "links:\n  a:b: d\n");
        let doc: Value = serde_yaml::from_str(&out).unwrap();
        assert_eq!(links_value(&doc)["a:b"].as_str(), Some("d"));
    }

    #[test]
    fn update_quoted_key_containing_colon() {
        // Quoted keys keep their quoting style through an in-place update.
        let raw = "links:\n  'a:b': c\n";
        let out = add_link_entry(raw, "a:b", "d").unwrap();
        assert_eq!(out, "links:\n  'a:b': d\n");
        let doc: Value = serde_yaml::from_str(&out).unwrap();
        assert_eq!(links_value(&doc)["a:b"].as_str(), Some("d"));
    }

    #[test]
    fn block_scalar_source_update_is_rejected() {
        // A block scalar spans following indented lines; replacing only the
        // indicator line would orphan its continuation lines.
        let raw = "links:\n  ~/.config/x: |\n    line1\n    line2\n";
        let result = add_link_entry(raw, "~/.config/x", "config/x");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("block/folded scalar"));
    }

    #[test]
    fn folded_scalar_source_update_is_rejected() {
        let raw = "links:\n  ~/.config/x: >-\n    line1\n";
        let result = add_link_entry(raw, "~/.config/x", "config/x");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("block/folded scalar"));
    }

    #[test]
    fn append_after_block_scalar_entry_is_rejected() {
        let raw = "links:\n  ~/.config/a: b\n  ~/.config/x: |\n    line1\n";
        let result = add_link_entry(raw, "~/.config/new", "config/new");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("block/folded scalar"));
    }

    #[test]
    fn empty_links_with_leading_comment_keeps_comment_above_entry() {
        let raw = "links:\n  # nothing yet\n";
        let out = add_link_entry(raw, "~/.config/example", "config/example").unwrap();
        assert_eq!(
            out,
            "links:\n  # nothing yet\n  ~/.config/example: config/example\n"
        );
    }

    #[test]
    fn crlf_line_endings_are_preserved() {
        let raw = "links:\r\n  a: b\r\ncreate: []\r\n";
        let out = add_link_entry(raw, "c", "d").unwrap();
        assert_eq!(out, "links:\r\n  a: b\r\n  c: d\r\ncreate: []\r\n");
        assert!(!out.replace("\r\n", "").contains('\r'));
    }

    #[test]
    fn crlf_replacement_keeps_crlf_endings() {
        let raw = "links:\r\n  a: b\r\n";
        let out = add_link_entry(raw, "a", "c").unwrap();
        assert_eq!(out, "links:\r\n  a: c\r\n");
    }

    #[test]
    fn split_key_value_handles_plain_and_quoted_keys() {
        assert_eq!(split_key_value("a: b"), Some(("a", " b")));
        assert_eq!(split_key_value("a:b: c"), Some(("a:b", " c")));
        assert_eq!(split_key_value("'a:b': c"), Some(("'a:b'", " c")));
        assert_eq!(split_key_value("\"a: b\": c"), Some(("\"a: b\"", " c")));
        assert_eq!(split_key_value("key:value"), None); // colon without space
        assert_eq!(split_key_value(""), None);
    }

    #[test]
    fn empty_inline_map_is_expanded() {
        let raw = "install: []\nlinks: {}\n";
        let out = add_link_entry(raw, "~/.config/example", "config/example").unwrap();
        assert_eq!(
            out,
            "install: []\nlinks:\n  ~/.config/example: config/example\n"
        );
    }

    #[test]
    fn empty_inline_list_is_expanded() {
        let raw = "links: []\n";
        let out = add_link_entry(raw, "~/.config/example", "config/example").unwrap();
        assert_eq!(
            out,
            "links:\n  - target: ~/.config/example\n    source: config/example\n"
        );
    }

    #[test]
    fn null_links_is_expanded() {
        let raw = "links: null\n";
        let out = add_link_entry(raw, "~/.config/example", "config/example").unwrap();
        assert_eq!(out, "links:\n  ~/.config/example: config/example\n");
    }

    #[test]
    fn missing_links_section_is_appended() {
        let raw = "install: []\n";
        let out = add_link_entry(raw, "~/.config/example", "config/example").unwrap();
        assert_eq!(
            out,
            "install: []\nlinks:\n  ~/.config/example: config/example\n"
        );
    }

    #[test]
    fn non_flow_values_are_quoted_when_needed() {
        let raw = "links: {}\n";
        let out = add_link_entry(raw, "a:b", "c # d").unwrap();
        // `a:b` is a legal plain scalar (colon not followed by a space), while
        // the comment-like source must be quoted.
        assert_eq!(out, "links:\n  a:b: 'c # d'\n");
        let doc: Value = serde_yaml::from_str(&out).unwrap();
        assert_eq!(links_value(&doc)["a:b"].as_str(), Some("c # d"));
    }

    #[test]
    fn non_empty_flow_map_is_rejected_without_editing() {
        let raw = "links: {a: b}\n";
        let result = add_link_entry(raw, "~/.config/example", "config/example");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("edit dotman.yaml manually"));
    }

    #[test]
    fn non_mapping_document_is_rejected() {
        let raw = "- item\n";
        let result = add_link_entry(raw, "~/.config/example", "config/example");
        assert!(result.is_err());
    }

    #[test]
    fn link_to_itself_is_not_added_twice() {
        let raw = "links:\n  a: b\n";
        let out = add_link_entry(raw, "a", "b2").unwrap();
        assert_eq!(out, "links:\n  a: b2\n");
        let doc: Value = serde_yaml::from_str(&out).unwrap();
        assert_eq!(links_value(&doc).as_mapping().unwrap().len(), 1);
    }

    #[test]
    fn no_trailing_newline_is_preserved() {
        let raw = "links:\n  a: b";
        let out = add_link_entry(raw, "c", "d").unwrap();
        assert_eq!(out, "links:\n  a: b\n  c: d");
    }

    #[test]
    fn missing_links_section_in_crlf_file_uses_crlf() {
        let raw = "install: []\r\n";
        let out = add_link_entry(raw, "~/.config/example", "config/example").unwrap();
        assert_eq!(
            out,
            "install: []\r\nlinks:\r\n  ~/.config/example: config/example\r\n"
        );
        // No stray \r outside the \r\n pairs.
        assert!(!out.replace("\r\n", "").contains('\r'));
    }

    #[test]
    fn duplicate_target_in_sequence_is_rejected_before_editing() {
        let raw = "links:\n  - target: a\n    source: x\n  - target: a\n    source: y\n";
        let result = add_link_entry(raw, "a", "z");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("appears multiple times"));
    }

    #[test]
    fn atomic_write_replaces_contents() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("dotman.yaml");
        std::fs::write(&path, "old").unwrap();
        write_config_atomic(&path, "new").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
        // No temporary files left behind.
        let leftovers: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp."))
            .collect();
        assert!(leftovers.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn atomic_write_preserves_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("dotman.yaml");
        std::fs::write(&path, "old").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        write_config_atomic(&path, "new").unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
    }

    #[cfg(unix)]
    #[test]
    fn atomic_write_keeps_symlink() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real.yaml");
        let link = dir.path().join("link.yaml");
        std::fs::write(&real, "old").unwrap();
        std::os::unix::fs::symlink(&real, &link).unwrap();
        write_config_atomic(&link, "new").unwrap();
        // The symlink itself survives and still points at the real file.
        assert!(
            std::fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(std::fs::read_link(&link).unwrap(), real);
        assert_eq!(std::fs::read_to_string(&real).unwrap(), "new");
    }

    #[test]
    fn split_value_comment_respects_quotes() {
        assert_eq!(split_value_comment("a # c"), ("a ", "# c"));
        assert_eq!(split_value_comment("'a # b' # c"), ("'a # b' ", "# c"));
        assert_eq!(split_value_comment("a#b"), ("a#b", ""));
    }
}
