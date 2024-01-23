use anyhow::Result;
use std::path::{Component, Path, PathBuf};

/// Converts `p` to a normalized path, but doesn't resolve symlinks. The function does normalize
/// the path by resolving any `.` and `..` components which are present. If `preserve_prefix_cur`
/// is `true`, a leading `./` of a relative path is preserved.
///
/// Use `to_lexical_absolute()` if you want to convert relative paths to absolute paths.
pub(crate) fn to_lexical_normal(p: &Path, preserve_prefix_cur: bool) -> PathBuf {
    let mut norm = PathBuf::new();
    for (i, component) in p.components().enumerate() {
        match component {
            Component::CurDir => {
                /* do nothing for `.` components */
                if i == 0 && preserve_prefix_cur {
                    norm.push(".");
                }
            }
            Component::ParentDir => {
                // pop the last element that we added for `..` components
                norm.pop();
            }
            // just push the component for any other component
            component => norm.push(component.as_os_str()),
        }
    }
    norm
}

/// Converts `p` to an absolute path, but doesn't resolve symlinks. The function does normalize the
/// path by resolving any `.` and `..` components which are present.
///
/// Copied from https://internals.rust-lang.org/t/path-to-lexical-absolute/14940.
pub(crate) fn to_lexical_absolute(p: &Path) -> Result<PathBuf> {
    let mut absolute = if p.is_absolute() {
        PathBuf::new()
    } else {
        std::env::current_dir()?
    };
    absolute.push(to_lexical_normal(p, false));
    Ok(absolute)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_lexical_absolute_for_absolute() {
        let orig = "/foo/bar/../bar/baz";
        let abs = to_lexical_absolute(&PathBuf::from(orig)).unwrap();
        assert_eq!(abs, PathBuf::from("/foo/bar/baz"));
    }

    #[test]
    fn test_to_lexical_normal_for_absolute() {
        let orig = "/foo/bar/../bar/baz";
        let abs = to_lexical_normal(&PathBuf::from(orig), false);
        assert_eq!(abs, PathBuf::from("/foo/bar/baz"));
    }

    #[test]
    fn test_to_lexical_absolute_for_relative() {
        let orig = "foo/bar/../bar/baz";
        let abs = to_lexical_absolute(&PathBuf::from(orig)).unwrap();
        let mut base = std::env::current_dir().unwrap();
        base.push("foo/bar/baz");
        assert_eq!(abs, base);
    }

    #[test]
    fn test_to_lexical_normal_for_relative() {
        let orig = "foo/bar/../bar/baz";
        let abs = to_lexical_normal(&PathBuf::from(orig), false);
        assert_eq!(abs, PathBuf::from("foo/bar/baz"));
        let orig = "./foo/bar/../bar/baz";
        let abs = to_lexical_normal(&PathBuf::from(orig), false);
        assert_eq!(abs, PathBuf::from("foo/bar/baz"));
    }

    #[test]
    fn test_to_lexical_normal_for_relative_preserve_dot_prefix() {
        let orig = "./foo/bar/../bar/baz";
        let abs = to_lexical_normal(&PathBuf::from(orig), true);
        assert_eq!(abs, PathBuf::from("./foo/bar/baz"));
    }
}
