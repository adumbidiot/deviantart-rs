/// Sanitize the given path, returning a santized path with only valid chars.
pub fn sanitize_path(path: &str) -> String {
    path.chars()
        .map(|c| {
            if [':', '?', '/', '|', '*'].contains(&c) {
                '-'
            } else {
                c
            }
        })
        .collect()
}
