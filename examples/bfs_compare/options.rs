const SIZES: [usize; 7] = [16, 32, 64, 128, 256, 512, 1024];

pub(super) const HELP: &str =
    "Usage: bfs_compare [--sizes LIST|all] [--runs N] [--seeds N] [--workload NAME]
  --sizes LIST|all   Comma-separated powers of two from 16 through 1024 (default: 16,32)
  --runs N           Timed runs per case (default: 9)
  --seeds N          Use sequential seeds 1 through N (default: 1)
  --workload NAME    Limit to open, backtracker, or multi (default: all three)
  --help             Show this help";

pub(super) struct Options {
    pub(super) sizes: Vec<usize>,
    pub(super) runs: usize,
    pub(super) seeds: usize,
    pub(super) workload: Option<String>,
}

pub(super) fn parse(mut args: impl Iterator<Item = String>) -> Result<Options, String> {
    let mut options = Options {
        sizes: Vec::new(),
        runs: 9,
        seeds: 1,
        workload: None,
    };
    while let Some(flag) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("{flag} needs a value"));
        match flag.as_str() {
            "--sizes" => {
                let list = value()?;
                options.sizes.clear();
                if list == "all" {
                    options.sizes.extend_from_slice(&SIZES);
                } else {
                    for item in list.split(',') {
                        let size = item.parse().ok().filter(|n| SIZES.contains(n)).ok_or_else(|| {
                            format!("invalid --sizes entry {item:?}; expected a power of two from 16 through 1024")
                        })?;
                        options.sizes.push(size);
                    }
                }
            }
            "--runs" | "--seeds" => {
                let raw = value()?;
                let n = raw.parse().ok().filter(|&n| n > 0).ok_or_else(|| {
                    format!("invalid {flag} value {raw:?}; expected a positive integer")
                })?;
                if flag == "--runs" {
                    options.runs = n;
                } else {
                    options.seeds = n;
                }
            }
            "--workload" => {
                let name = value()?;
                if !matches!(name.as_str(), "open" | "backtracker" | "multi") {
                    return Err(format!(
                        "invalid --workload {name:?}; expected open, backtracker, or multi"
                    ));
                }
                options.workload = Some(name);
            }
            _ => return Err(format!("unknown argument {flag:?}")),
        }
    }
    if options.sizes.is_empty() {
        options.sizes.extend_from_slice(&SIZES[..2]);
    }
    Ok(options)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(line: &str) -> Result<Options, String> {
        parse(line.split_whitespace().map(str::to_owned))
    }

    #[test]
    fn defaults() {
        let opts = options("").unwrap();
        assert_eq!(opts.sizes, [16, 32]);
        assert_eq!((opts.runs, opts.seeds), (9, 1));
        assert_eq!(opts.workload, None);
    }

    #[test]
    fn all_sizes() {
        assert_eq!(
            options("--sizes all").unwrap().sizes,
            [16, 32, 64, 128, 256, 512, 1024]
        );
    }

    #[test]
    fn explicit_options() {
        let opts = options("--sizes 16,512,1024 --seeds 3 --workload multi --runs 4").unwrap();
        assert_eq!(opts.sizes, [16, 512, 1024]);
        assert_eq!((opts.runs, opts.seeds), (4, 3));
        assert_eq!(opts.workload.as_deref(), Some("multi"));
    }

    #[test]
    fn invalid_input() {
        for input in [
            "--sizes 8",
            "--sizes 48",
            "--sizes 2048",
            "--sizes 16,",
            "--sizes bad",
            "--sizes",
            "--runs 0",
            "--seeds 0",
            "--runs -1",
            "--workload other",
            "--transformer",
        ] {
            assert!(options(input).is_err(), "accepted {input:?}");
        }
    }
}
