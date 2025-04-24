# Simple Ram Cleaner

Utilitaire simple et efficace pour nettoyer la mémoire cache des processus Windows et libérer la RAM.

## Fonctionnalités

- Interface graphique moderne et intuitive avec thème sombre
- Logo Nukleos intégré dans l'interface et comme icône d'application
- Nettoyage rapide de la mémoire cache des processus Windows
- Libération de la mémoire système inutilisée
- Affichage détaillé des résultats de nettoyage
- Suivi en temps réel du nettoyage avec barre de progression
- Statistiques sur l'état de la mémoire avant/après nettoyage
- Tri des processus par quantité de mémoire libérée
- Application en mode GUI pur (sans fenêtre console)

## Roadmap

- Nettoyage de la VRAM (support NVIDIA + AMD)

## Installation

### Prérequis

- Windows 10/11
- Droits administrateur (obligatoire pour le nettoyage de la mémoire)

### Téléchargement

Téléchargez la dernière version sur la page des [releases](https://github.com/zehelh/simple_ram_cleaner/releases).

### Compilation

Pour compiler le projet depuis les sources :

1. Installez Rust (https://www.rust-lang.org/tools/install)
2. Clonez ce dépôt
3. Exécutez `cargo build --release`
4. L'exécutable se trouvera dans `./target/x86_64-pc-windows-msvc/release/simple_ram_cleaner.exe`

## Utilisation

Lancez l'application en double-cliquant sur l'exécutable. L'application doit être exécutée avec des privilèges administrateur pour fonctionner correctement.

Une fois l'application ouverte, cliquez simplement sur le bouton "Nettoyer la mémoire cache" pour lancer le processus. Une barre de progression indiquera l'avancement et les résultats s'afficheront automatiquement une fois le nettoyage terminé.

## Notes importantes

- **Cette application nécessite des privilèges administrateur pour fonctionner correctement.**
- Le nettoyage de la mémoire cache peut affecter temporairement les performances des applications, car les données doivent être rechargées.

## Licence

Ce projet est sous licence MIT - voir le fichier LICENSE pour plus de détails.

## Version

Version actuelle: 1.0.0 