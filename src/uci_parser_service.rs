

pub struct UciParserService;

impl UciParserService {

    pub fn new() -> Self {
        UciParserService {}
    }

    /// Got a "go" command and return wtime and btime in ms. (-1, -1) if no time information given.
    pub fn parse_go(&self, command: &str) -> (i32, i32) {

        let mut wtime = -1;
        let mut btime = -1;

        if !command.contains("wtime") || !command.contains("btime") {
            return (wtime, btime);
        }

        let command_parts: Vec<&str> = command.split_whitespace().collect();

        let mut iter = command_parts.iter();
        while let Some(part) = iter.next() {
            match *part {
                "wtime" => {
                    if let Some(value) = iter.next() {
                        wtime = value.parse().unwrap_or(-1);
                    }
                },
                "btime" => {
                    if let Some(value) = iter.next() {
                        btime = value.parse().unwrap_or(-1);
                    }
                },
                _ => {}
            }
       }
        (wtime, btime)
    }

}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_go_valid_times() {
        let parser = UciParserService {};
        let command = "go wtime 31520 btime 1410";
        let (wtime, btime) = parser.parse_go(command);
        assert_eq!(31520, wtime);
        assert_eq!(1410, btime);

        let parser = UciParserService {};
        let command = "go";
        let (wtime, btime) = parser.parse_go(command);
        assert_eq!(-1, wtime);
        assert_eq!(-1, btime);

        let parser = UciParserService {};
        let command = "go wtime 31520 btime 1410 something extra";
        let (wtime, btime) = parser.parse_go(command);
        assert_eq!(31520, wtime);
        assert_eq!(1410, btime);
    }

}
