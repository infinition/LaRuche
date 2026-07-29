//! Embarque l'icone dans l'executable Windows.
//!
//! Sans cela, le `.ico` genere par `laruche-icones` resterait un fichier inutilise:
//! Windows lit l'icone dans les RESSOURCES du binaire, pas a cote de lui. C'est ce
//! qui donne une icone distincte dans la barre des taches et le gestionnaire de
//! taches, au lieu de la vignette generique d'un exe sans ressource.
fn main() {
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=icone.ico");
        if std::path::Path::new("icone.ico").exists() {
            let mut res = winresource::WindowsResource::new();
            res.set_icon("icone.ico");
            // Une icone manquante ou un outil absent ne doit pas empecher de compiler:
            // on previent et on continue avec le binaire sans ressource.
            if let Err(e) = res.compile() {
                println!("cargo:warning=icone non embarquee: {e}");
            }
        }
    }
}
