use std::collections::VecDeque;
use std::env::args;
use std::fs::read_dir;
use std::path::PathBuf;

fn main() {
    let argv: Vec<_> = args().collect();
    assert_eq!(argv.len(), 2);
    let mut queue = VecDeque::new();
    queue.push_back(PathBuf::from(&argv[1]));
    while !queue.is_empty() {
        println!("{:?}", &queue);
        let mut entries: Vec<_> =
            read_dir(queue.pop_front().unwrap())
            .unwrap()
            .map(|x| match x {
                Ok(e) => (e.file_name(), e),
                Err(e) => panic!("{:?}", e),
            })
            .collect();

        // TODO: Guarantee that Ord for OsString will never change, then
        // document its behavior.
        entries.sort_by(|x, y| x.0.cmp(&y.0));

        for entry in entries {
            let file_type = entry.1.file_type().unwrap();
            if file_type.is_file() {
                println!("{:?}", entry.0);
            } else if file_type.is_dir() {
                println!("{:?}/", entry.0);
                queue.push_back(entry.1.path());
            }
        }
    }
}
