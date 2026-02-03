use std::env;
use std::fs;
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use flate2::write::ZlibEncoder;
use flate2::Compression;
use sha1::{Sha1, Digest};

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <command>", args[0]);
        std::process::exit(1);
    }

    match args[1].as_str() {
        "init" => {
            fs::create_dir_all(".git/objects")?;
            fs::create_dir_all(".git/refs")?;

            // HEAD usually points to main by default
            if fs::metadata(".git/HEAD").is_err() {
                fs::write(".git/HEAD", "ref: refs/heads/main\n")?;
            }

            println!("Initialized git directory");
        }

        "cat-file" => {
            if args.len() < 4 || args[2] != "-p" {
                eprintln!("Usage: {} cat-file -p <sha>", args[0]);
                std::process::exit(1);
            }

            let sha = &args[3];
            if sha.len() != 40 {
                eprintln!("Invalid SHA-1 hash length");
                std::process::exit(1);
            }

            // Git objects are stored in .git/objects/../..
            let dir = &sha[..2];
            let file = &sha[2..];
            let path = format!(".git/objects/{}/{}", dir, file);

            let f = File::open(path)?;
            let mut z = flate2::read::ZlibDecoder::new(f);
            let mut decompressed = Vec::new();
            z.read_to_end(&mut decompressed)?;

            // The format is "<type> <size>\0<content>"
            // We just want to print the content after the null byte.
            if let Some(null_pos) = decompressed.iter().position(|&b| b == 0) {
                let content = &decompressed[null_pos + 1..];
                io::stdout().write_all(content)?;
            }
        }

        "hash-object" => {
            if args.len() < 3 {
                eprintln!("Usage: {} hash-object [-w] <file>", args[0]);
                std::process::exit(1);
            }

            let mut write = false;
            let file_arg;

            if args[2] == "-w" {
                write = true;
                if args.len() != 4 {
                    eprintln!("Usage: {} hash-object -w <file>", args[0]);
                    std::process::exit(1);
                }
                file_arg = &args[3];
            } else {
                file_arg = &args[2];
            }

            let mut file = File::open(file_arg)?;
            let mut content = Vec::new();
            file.read_to_end(&mut content)?;

            // Standard blob header: "blob <size>\0"
            let header = format!("blob {}\0", content.len());
            let mut object = Vec::new();
            object.extend_from_slice(header.as_bytes());
            object.extend_from_slice(&content);

            let mut hasher = Sha1::new();
            hasher.update(&object);
            let sha = hasher.finalize();
            let sha_hex = format!("{:x}", sha);

            if write {
                write_object(&sha_hex, &object)?;
            }

            println!("{}", sha_hex);
        }

        "ls-tree" => {
            if args.len() < 4 || args[2] != "--name-only" {
                eprintln!("Usage: {} ls-tree --name-only <tree_sha>", args[0]);
                std::process::exit(1);
            }

            let sha = &args[3];
            let dir = &sha[..2];
            let file = &sha[2..];
            let path = format!(".git/objects/{}/{}", dir, file);

            let f = File::open(path)?;
            let mut z = flate2::read::ZlibDecoder::new(f);
            let mut decompressed = Vec::new();
            z.read_to_end(&mut decompressed)?;

            // Tree format: [mode] [name]\0[20 byte sha]
            // We need to parse this repeatedly until we run out of data.
            if let Some(null_pos) = decompressed.iter().position(|&b| b == 0) {
                let mut content = &decompressed[null_pos + 1..];

                while !content.is_empty() {
                    if let Some(null_idx) = content.iter().position(|&b| b == 0) {
                        let mode_and_name = &content[..null_idx];

                        if let Some(space_idx) = mode_and_name.iter().position(|&b| b == b' ') {
                            let name = String::from_utf8_lossy(&mode_and_name[space_idx + 1..]);
                            println!("{}", name);
                        }

                        // Skip the null byte + 20 bytes of SHA to get to the next entry
                        content = &content[null_idx + 1 + 20..];
                    } else {
                        break;
                    }
                }
            }
        }

        "write-tree" => {
            let tree_sha = write_tree_recursive(".")?;
            println!("{}", tree_sha);
        }

        "commit-tree" => {
            if args.len() < 6 {
                eprintln!("Usage: {} commit-tree <tree_sha> -p <parent_sha> -m <message>", args[0]);
                std::process::exit(1);
            }

            let tree_sha = &args[2];
            let mut parent_sha = String::new();
            let mut message = String::new();

            // Simple parser for the flags
            let mut i = 3;
            while i < args.len() {
                match args[i].as_str() {
                    "-p" => {
                        if i + 1 < args.len() {
                            parent_sha = args[i + 1].clone();
                            i += 2;
                        } else {
                            eprintln!("Error: -p requires a parent SHA");
                            std::process::exit(1);
                        }
                    },
                    "-m" => {
                        if i + 1 < args.len() {
                            message = args[i + 1].clone();
                            i += 2;
                        } else {
                            eprintln!("Error: -m requires a message");
                            std::process::exit(1);
                        }
                    },
                    _ => i += 1,
                }
            }

            let commit_sha = create_commit(tree_sha, &parent_sha, &message)?;
            println!("{}", commit_sha);
        }

        other => {
            println!("unknown command: {}", other);
        }
    }

    Ok(())
}

fn write_tree_recursive(dir_path: &str) -> io::Result<String> {
    let path = Path::new(dir_path);
    let mut entries = Vec::new();

    let dir_entries = fs::read_dir(path)?;

    for entry in dir_entries {
        let entry = entry?;
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy();

        // Must ignore .git or we'll recurse infinitely into the database itself
        if file_name_str == ".git" {
            continue;
        }

        let entry_path = entry.path();
        let metadata = entry.metadata()?;

        if metadata.is_file() {
            let mut file = File::open(&entry_path)?;
            let mut content = Vec::new();
            file.read_to_end(&mut content)?;

            // Git checks execution permissions.
            // 100755 for executable, 100644 for regular files.
            let mode = if metadata.permissions().mode() & 0o111 != 0 {
                "100755"
            } else {
                "100644"
            };

            let header = format!("blob {}\0", content.len());
            let mut object = Vec::new();
            object.extend_from_slice(header.as_bytes());
            object.extend_from_slice(&content);

            let mut hasher = Sha1::new();
            hasher.update(&object);
            let sha = hasher.finalize();
            let sha_hex = format!("{:x}", sha);

            write_object(&sha_hex, &object)?;

            let sha_bytes = hex_to_bytes(&sha_hex);

            entries.push((
                mode.to_string(),
                file_name_str.to_string(),
                sha_bytes,
            ));

        } else if metadata.is_dir() {
            let entry_path_str = entry_path.to_string_lossy();
            let tree_sha = write_tree_recursive(&entry_path_str)?;
            
            // Git doesn't track empty directories.
            // This is the SHA for an empty tree. If we see it, skip adding it.
            if tree_sha == "4b825dc642cb6eb9a060e54bf8d69288fbee4904" {
                continue; 
            }

            let sha_bytes = hex_to_bytes(&tree_sha);

            entries.push((
                "40000".to_string(),
                file_name_str.to_string(),
                sha_bytes,
            ));
        }
    }

    // Git requires tree entries to be sorted by name for the hash to be consistent
    entries.sort_by(|a, b| a.1.cmp(&b.1));

    let mut tree_content = Vec::new();
    for (mode, name, sha_bytes) in entries {
        tree_content.extend_from_slice(mode.as_bytes());
        tree_content.push(b' ');
        tree_content.extend_from_slice(name.as_bytes());
        tree_content.push(0); // Null byte separator
        tree_content.extend_from_slice(&sha_bytes);
    }

    let header = format!("tree {}\0", tree_content.len());
    let mut tree_object = Vec::new();
    tree_object.extend_from_slice(header.as_bytes());
    tree_object.extend_from_slice(&tree_content);

    let mut hasher = Sha1::new();
    hasher.update(&tree_object);
    let tree_sha = hasher.finalize();
    let tree_sha_hex = format!("{:x}", tree_sha);

    write_object(&tree_sha_hex, &tree_object)?;

    Ok(tree_sha_hex)
}

fn create_commit(tree_sha: &str, parent_sha: &str, message: &str) -> io::Result<String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Hardcoded author for this assignment
    let author_name = "Romain";
    let author_email = "romain@mail.com";
    let timezone = "+0100";

    let mut commit_content = String::new();
    commit_content.push_str(&format!("tree {}\n", tree_sha));

    if !parent_sha.is_empty() {
        commit_content.push_str(&format!("parent {}\n", parent_sha));
    }

    // Format: "author Name <email> timestamp timezone"
    commit_content.push_str(&format!(
        "author {} <{}> {} {}\n",
        author_name, author_email, timestamp, timezone
    ));
    commit_content.push_str(&format!(
        "committer {} <{}> {} {}\n",
        author_name, author_email, timestamp, timezone
    ));
    commit_content.push_str(&format!("\n{}\n", message));

    let header = format!("commit {}\0", commit_content.len());
    let mut commit_object = Vec::new();
    commit_object.extend_from_slice(header.as_bytes());
    commit_object.extend_from_slice(commit_content.as_bytes());

    let mut hasher = Sha1::new();
    hasher.update(&commit_object);
    let commit_sha = hasher.finalize();
    let commit_sha_hex = format!("{:x}", commit_sha);

    write_object(&commit_sha_hex, &commit_object)?;

    Ok(commit_sha_hex)
}

// Helper to write compressed objects to the .git/objects folder
fn write_object(sha_hex: &str, object: &[u8]) -> io::Result<()> {
    let dir = &sha_hex[..2];
    let file = &sha_hex[2..];
    let path = format!(".git/objects/{}/{}", dir, file);

    // Optimization: If the object already exists, don't overwrite it.
    // This also avoids permission errors with read-only files created by git.
    if Path::new(&path).exists() {
        return Ok(());
    }

    fs::create_dir_all(format!(".git/objects/{}", dir))?;

    let f = File::create(path)?;
    let mut encoder = ZlibEncoder::new(f, Compression::default());
    encoder.write_all(object)?;
    encoder.finish()?;

    Ok(())
}

fn hex_to_bytes(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}