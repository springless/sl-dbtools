/// As part of printing an error string for a SQL query we'd like to print out the
/// relevant query context including the line number, but all we get back from Postgres
/// is the character number, so we take the original query string and break it down into
/// lines and figure out which line the position index is on.
pub fn get_query_pos_str(query: &str, pos: usize) -> String {
    let mut cur_line_num = 0;
    let mut cur_pos = 0;
    for cur_line in query.split('\n') {
        cur_line_num += 1;
        let next_pos: usize = cur_pos + cur_line.len() + 1; // +1 to account for `\n`
        let line_num_str = format!("LINE {}", cur_line_num);
        if pos >= cur_pos && pos < next_pos {
            return format!(
                "{} | {}\n{} | {}^",
                line_num_str,
                cur_line,
                " ".repeat(line_num_str.len()),
                " ".repeat(pos - cur_pos - 1), // -1 because the carat should appear *at* the
                                               // position
            );
        }
        cur_pos = next_pos;
    }
    "Not found".to_string()
}
