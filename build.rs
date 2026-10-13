use std::fs;
use std::path::Path;

fn main() {

    let files_to_copy = ["students.xlsx", "tickets.docx"];
    
    let target_dirs = [
        "test_env",
        "test_env/ui_e2e_fixture",
        "target/debug", 
    ];

    for file in &files_to_copy {
        if Path::new(file).exists() {
            for dir in &target_dirs {
                let _ = fs::create_dir_all(dir);
                let dest = format!("{}/{}", dir, file);
                let _ = fs::copy(file, &dest);
            }
        }
    }

    println!("cargo:rerun-if-changed=students.xlsx");
    println!("cargo:rerun-if-changed=tickets.docx");
}