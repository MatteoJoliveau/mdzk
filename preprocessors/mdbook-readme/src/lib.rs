use regex::Captures;
use lazy_regex::regex;
use mdbook::errors::*;
use mdbook::preprocess::{Preprocessor, PreprocessorContext};
use mdbook::book::{Book, BookItem};
use std::path::Path;
use log::{warn};

/// A preprocessor for converting file name `README.md` to `index.md` since
/// `README.md` is the de facto index file in markdown-based documentation.
#[derive(Default)]
pub struct ReadmePreprocessor;

impl ReadmePreprocessor {
    /// Create a new `IndexPreprocessor`.
    pub fn new() -> Self {
        ReadmePreprocessor
    }
}

impl Preprocessor for ReadmePreprocessor {
    fn name(&self) -> &str {
        "readme"
    }

    fn run(&self, ctx: &PreprocessorContext, mut book: Book) -> Result<Book> {
        let readme_link_re = regex!(r"(?i)\[(.*?)\]\(<?README(?:(?:\.md)|(?:\.markdown))>?\)");

        let source_dir = ctx.root.join(&ctx.config.book.src);
        book.for_each_mut(|section: &mut BookItem| {
            if let BookItem::Chapter(ref mut ch) = *section {
                if let Some(ref mut path) = ch.path {
                    if is_readme_file(&path) {
                        let mut index_md = source_dir.join(path.with_file_name("index.md"));
                        if index_md.exists() {
                            warn_readme_name_conflict(&path, &&mut index_md);
                        }
                        path.set_file_name("index.md");
                    }  else {
                        ch.content = readme_link_re.replace_all(&ch.content, |caps: &Captures| {
                            format!("[{}](index.md)", &caps[1])
                        }).to_string();
                    }
                }
            }
        });

        Ok(book)
    }
}

fn warn_readme_name_conflict<P: AsRef<Path>>(readme_path: P, index_path: P) {
    let file_name = readme_path.as_ref().file_name().unwrap_or_default();
    let parent_dir = index_path
        .as_ref()
        .parent()
        .unwrap_or_else(|| index_path.as_ref());
    warn!(
        "It seems that there are both {:?} and index.md under \"{}\".",
        file_name,
        parent_dir.display()
    );
    warn!(
        "mdbook converts {:?} into index.html by default. It may cause",
        file_name
    );
    warn!("unexpected behavior if putting both files under the same directory.");
    warn!("To solve the warning, try to rearrange the book structure or disable");
    warn!("\"index\" preprocessor to stop the conversion.");
}

fn is_readme_file<P: AsRef<Path>>(path: P) -> bool {
    regex!(r"(?i)^readme$").is_match(
        path.as_ref()
            .file_stem()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or_default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_stem_exactly_matches_readme_case_insensitively() {
        let path = "path/to/Readme.md";
        assert!(is_readme_file(path));

        let path = "path/to/README.md";
        assert!(is_readme_file(path));

        let path = "path/to/rEaDmE.md";
        assert!(is_readme_file(path));

        let path = "path/to/README.markdown";
        assert!(is_readme_file(path));

        let path = "path/to/README";
        assert!(is_readme_file(path));

        let path = "path/to/README-README.md";
        assert!(!is_readme_file(path));
    }
}