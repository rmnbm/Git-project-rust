// Permet d'importer des fonctions pour interagir avec le système et gérer les fichiers
use std::env; // Pour récupérer les arguments passés au programme
use std::fs;  // Pour créer des dossiers et écrire des fichiers

use std::fs::File;                   // Pour ouvrir les fichiers blob
use std::io::{self, Read, Write};    // Pour lire/écrire le contenu des fichiers
use flate2::read::ZlibDecoder;       // Pour décompresser les blobs compressés par Git

fn main() -> io::Result<()> { // On retourne un Result pour pouvoir utiliser le "?" et propager les erreurs
    let args: Vec<String> = env::args().collect(); // Récupère tous les arguments passés à ton programme

    // Si aucun argument, on affiche un message d'erreur
    if args.len() < 2 {
        eprintln!("Usage: {} <command>", args[0]);
        std::process::exit(1);
    }

    match args[1].as_str() { // On regarde la première commande passée (init, cat-file, etc.)
        "init" => {
            // Ici, on initialise un mini dépôt Git
            // On vérifie si les dossiers existent avant de les créer pour éviter le panic
            if !fs::metadata(".git").is_ok() {
                fs::create_dir(".git").unwrap();
            }
            if !fs::metadata(".git/objects").is_ok() {
                fs::create_dir(".git/objects").unwrap();
            }
            if !fs::metadata(".git/refs").is_ok() {
                fs::create_dir(".git/refs").unwrap();
            }

            // On crée le fichier HEAD qui pointe par défaut sur la branche main
            fs::write(".git/HEAD", "ref: refs/heads/main\n").unwrap();

            println!("Initialized git directory"); // Message pour l’utilisateur
        }

        "cat-file" => {
            // On s'assure que l'utilisateur a passé les bons arguments : cat-file -p <sha>
            if args.len() != 4 || args[2] != "-p" {
                eprintln!("Usage: {} cat-file -p <sha>", args[0]);
                std::process::exit(1);
            }

            let sha = &args[3];                   // Récupère le SHA-1 du blob
            let dir = &sha[..2];                  // Les 2 premiers caractères = dossier dans .git/objects
            let file = &sha[2..];                 // Le reste = nom du fichier
            let path = format!(".git/objects/{}/{}", dir, file); // Construit le chemin complet vers le blob

            let f = File::open(path)?;            // Ouvre le fichier compressé
            let mut z = ZlibDecoder::new(f);      // Décompresse le fichier zlib
            let mut decompressed = Vec::new();
            z.read_to_end(&mut decompressed)?;    // Lit tout le contenu décompressé dans un vecteur

            // Le format d'un blob Git : "blob <taille>\0<contenu>"
            // On cherche la position du premier 0 (séparateur)
            if let Some(null_pos) = decompressed.iter().position(|&b| b == 0) {
                let content = &decompressed[null_pos + 1..]; // On récupère seulement le contenu réel
                io::stdout().write_all(content)?;           // On l'affiche sur la console
            }
        }

        other => { // Si la commande n'est pas reconnue
            println!("unknown command: {}", other);
        }
    }

    Ok(()) // Fin du programme
}