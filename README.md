# CQLRS - Cassandra CLI Client in Rust

Ein vollständiger, funktionaler Cassandra CLI-Client, geschrieben in Rust.

## Features

✨ **Vollständige CQL-Unterstützung**
- Ausführung beliebiger CQL-Queries
- Interaktiver REPL-Modus mit History
- Multi-Line-Query-Unterstützung
- Batch-Ausführung aus Dateien

🎨 **Flexible Ausgabeformate**
- Tabellenformat (Standard, mit schöner Box-Darstellung)
- JSON-Format
- CSV-Format

🔐 **Authentifizierung & Sicherheit**
- Username/Password-Authentifizierung
- Sichere Passworteingabe (ohne bash_history)
- SSL/TLS-Verschlüsselung
- Eigene CA-Zertifikate
- Mehrere Hosts (Load Balancing)
- Keyspace-Auswahl
- Konfigurierbare Ports

🚀 **Performance & Usability**
- Asynchrone Operationen mit Tokio
- Command History
- Farbige Ausgabe
- Fehlerbehandlung

## Installation

### Voraussetzungen
- Rust 1.70+ (installiere über [rustup](https://rustup.rs/))
- Zugang zu einem Cassandra/ScyllaDB Cluster

### Build
```bash
cargo build --release
```

Die Binary findet sich dann unter `target/release/cqlrs`.

## Verwendung

### Interaktiver Modus (REPL)
```bash
# Einfacher Start (localhost:9042)
cqlrs

# Mit spezifischem Host
cqlrs --hosts 192.168.1.100

# Mit Authentifizierung (sichere Passworteingabe)
cqlrs --hosts cassandra.example.com --username myuser -P

# Mit Authentifizierung (Passwort als Argument - nicht empfohlen)
cqlrs --hosts cassandra.example.com --username myuser --password mypass

# Mit SSL/TLS (ohne Zertifikatsvalidierung - Standard)
cqlrs --hosts cassandra.example.com --ssl --username myuser -P

# Mit SSL/TLS und Zertifikatsvalidierung
cqlrs --hosts cassandra.example.com --ssl --ssl-verify --username myuser -P

# Mit SSL und eigenem Zertifikat
cqlrs --hosts cassandra.example.com --ssl --ssl-ca-cert /path/to/ca.crt --username myuser -P

# Mit Keyspace
cqlrs --keyspace my_keyspace

# Mehrere Hosts (für Load Balancing)
cqlrs --hosts "host1.example.com,host2.example.com,host3.example.com"
```

### Einzelne Query ausführen
```bash
cqlrs --execute "SELECT * FROM system.local;"

cqlrs -e "SELECT * FROM my_keyspace.my_table LIMIT 10;" --output-format json
```

### Queries aus Datei ausführen
```bash
cqlrs --file queries.cql

# Mit JSON-Ausgabe
cqlrs --file migrations.cql --output-format json
```

### REPL-Befehle

Im interaktiven Modus stehen folgende Befehle zur Verfügung:

#### System-Befehle
- `help` - Zeigt Hilfe an
- `quit` / `exit` - Beendet den Client
- `clear` - Löscht den Bildschirm
- `\format <format>` - Ändert Ausgabeformat (table, json, csv)

#### Schnell-Befehle
- `\dk` - Listet alle Keyspaces
- `\dt` - Listet alle Tabellen
- `\dt <keyspace>` - Listet Tabellen in einem Keyspace
- `export <table> [datei.csv]` - Exportiert alle Tabellenzeilen nach CSV (Standard: `<keyspace>_<table>.csv`)
- `import <table> [datei.csv]` - Importiert CSV-Zeilen in die Tabelle (Standard: `<keyspace>_<table>.csv`)

#### CQL-Queries
Alle CQL-Befehle werden mit `;` abgeschlossen:

```sql
-- Keyspace verwenden
USE my_keyspace;

-- Daten abfragen
SELECT * FROM users WHERE id = 123;

-- Multi-Line-Query
SELECT user_id, name, email
FROM users
WHERE created_at > '2024-01-01'
LIMIT 100;

-- Daten einfügen
INSERT INTO users (id, name, email) 
VALUES (uuid(), 'Max Mustermann', 'max@example.com');

-- Schema abfragen
DESCRIBE KEYSPACES;
DESCRIBE TABLES;
```

## Beispiele

### Keyspace erstellen und verwenden
```bash
cqlrs -e "CREATE KEYSPACE IF NOT EXISTS test_ks WITH replication = {'class': 'SimpleStrategy', 'replication_factor': 1};"

cqlrs --keyspace test_ks
```

### Tabelle erstellen und Daten einfügen
```sql
cqlrs> CREATE TABLE users (
    ->   id uuid PRIMARY KEY,
    ->   name text,
    ->   email text,
    ->   created_at timestamp
    -> );

cqlrs> INSERT INTO users (id, name, email, created_at) 
    -> VALUES (uuid(), 'Alice', 'alice@example.com', toTimestamp(now()));

cqlrs> SELECT * FROM users;
```

### JSON-Export
```bash
cqlrs --keyspace my_keyspace \
      --execute "SELECT * FROM users;" \
      --output-format json > users.json
```

### Batch-Migration
Erstelle eine Datei `migration.cql`:
```sql
CREATE KEYSPACE IF NOT EXISTS production 
WITH replication = {'class': 'NetworkTopologyStrategy', 'dc1': 3};

USE production;

CREATE TABLE IF NOT EXISTS users (
    id uuid PRIMARY KEY,
    username text,
    email text,
    created_at timestamp
);

CREATE INDEX IF NOT EXISTS users_email_idx ON users (email);
```

Ausführen:
```bash
cqlrs --file migration.cql
```

## Kommandozeilen-Optionen

| Option | Kurzform | Beschreibung | Standard |
|--------|----------|-------------|----------|
| `--hosts` | `-h` | Cassandra-Hosts (komma-separiert) | `127.0.0.1` |
| `--port` | `-p` | Port | `9042` |
| `--username` | `-u` | Benutzername | - |
| `--password-prompt` | `-P` | Passwort-Eingabeaufforderung (empfohlen) | `false` |
| `--password` | - | Passwort direkt (nicht empfohlen) | - |
| `--keyspace` | `-k` | Zu verwendender Keyspace | - |
| `--ssl` | - | SSL/TLS aktivieren | `false` |
| `--ssl-ca-cert` | - | Pfad zum CA-Zertifikat | - |
| `--ssl-verify` | - | SSL-Zertifikat verifizieren | `true` |
| `--execute` | `-e` | Einzelne Query ausführen | - |
| `--file` | `-f` | Queries aus Datei ausführen | - |
| `--output-format` | `-o` | Ausgabeformat (table/json/csv) | `table` |
| `--verbose` | `-v` | Verbose Logging | `false` |

## Entwicklung

### Tests ausführen
```bash
cargo test
```

### Mit Logging
```bash
RUST_LOG=debug cargo run
```

### Entwicklung mit lokalem Cassandra
```bash
# Cassandra in Docker starten
docker run -d --name cassandra -p 9042:9042 cassandra:latest

# Client verbinden
cargo run
```

## Technologie-Stack

- **[scylla](https://github.com/scylladb/scylla-rust-driver)** - High-Performance Cassandra/ScyllaDB Driver
- **[clap](https://github.com/clap-rs/clap)** - Command-line Argument Parser
- **[tokio](https://tokio.rs/)** - Async Runtime
- **[rustyline](https://github.com/kkawakam/rustyline)** - Readline Implementation für REPL
- **[prettytable-rs](https://github.com/phsym/prettytable-rs)** - Tabellenformatierung
- **[colored](https://github.com/mackwic/colored)** - Farbige Terminal-Ausgabe



