# GitSearcher - TODO

Tool CLI in C# (.NET) per cercare ricorsivamente tutti i repository git sotto un percorso e generare un report JSON con posizione e data dell'ultima attività.

## Funzionalità richieste
- [x] Accettare un percorso come argomento (opzionale: default = directory corrente)
- [x] Scansione ricorsiva alla ricerca di cartelle `.git`
- [x] Determinare la data dell'ultima attività git per ogni repository
- [x] Esportare i risultati in un file JSON
- [x] Compilare in eseguibile `.exe` autonomo (Windows x64)

## Dettagli implementativi
- [x] Progetto console .NET (`GitSearcher.csproj`)
- [x] Argomenti CLI:
  - `[path]` (posizionale, opzionale)
  - `-o, --output <file>` per scegliere il file JSON di output (default: `git-repos.json` nella dir corrente)
  - `--include-hidden` per includere cartelle nascoste/`node_modules`
- [x] Per ogni `.git` trovato:
  - [x] percorso assoluto del repository (cartella che contiene `.git`)
  - [x] data ultima attività (priorità: timestamp `.git/logs/HEAD` → file più recente in `.git/refs` → ultima modifica di `.git`)
  - [x] eventuale branch corrente (da `.git/HEAD`)
- [x] Gestione errori: cartelle senza permessi vengono saltate con log a stderr
- [x] Skip di default: `node_modules`, cartelle che iniziano con `.` diverse da `.git`
- [x] Output JSON ordinato per data discendente

## Build
- [x] `dotnet publish -c Release -r win-x64 --self-contained false` (framework-dependent, snello)
- [ ] Opzionale: `--self-contained true /p:PublishSingleFile=true` per binario unico distribuibile

## Uso
```
GitSearcher.exe                       # scansiona la cartella corrente
GitSearcher.exe C:\Dev                # scansiona C:\Dev
GitSearcher.exe C:\Dev -o repos.json  # scansiona e salva in repos.json
```

## GUI Rust (`ui/`)
- [x] Progetto `gitsearcher-ui` con eframe/egui
- [x] File di config TOML (`gitsearcher-ui.toml`) cercato accanto all'exe e nel cwd
  - `executable` (default `"GitSearcher.exe"` → funziona se è nel PATH)
  - `extra_args`, `default_path`, `include_hidden`
  - Path relativi risolti rispetto al file di config o alla UI exe; assoluti rispettati; fallback al PATH di sistema
- [x] Risoluzione executable robusta: assoluto → relativo a config → relativo a UI → PATH
- [x] Input percorso + dialogo "Sfoglia…" (rfd)
- [x] Scansione su thread separato, log stderr in tempo reale
- [x] Tabella ordinabile (egui_extras) con filtro, doppio clic apre in Esplora risorse
- [x] Mostra branch, data ultima attività (orario locale), fonte
- [x] Costruito in release: `ui/target/release/gitsearcher-ui.exe`

## Esempio output JSON
```json
[
  {
    "path": "C:\\Dev\\MyRepo",
    "branch": "main",
    "lastActivityUtc": "2026-05-21T09:12:43Z",
    "source": "logs/HEAD"
  }
]
```
