# TODO — Pubblicazione su winget

## 1. Preparare il binario
- [ ] Build self-contained win-x64:
  ```
  dotnet publish -c Release -r win-x64 --self-contained true -p:PublishSingleFile=true
  ```
- [ ] Testare il `.exe` risultante su una macchina pulita (senza .NET installato)
- [ ] Decidere se distribuire solo il backend C# o anche la UI Rust (o uno zip con entrambi)

## 2. GitHub Release
- [ ] Scegliere un `PackageIdentifier` univoco — convenzione: `<Editore>.<AppName>` es. `Parresia.GitSearcher`
- [ ] Creare un tag semantico (`v1.0.0`) e una Release su GitHub
- [ ] Build self-contained linux-x64:
  ```
  dotnet publish -c Release -r linux-x64 --self-contained true -p:PublishSingleFile=true
  ```
- [ ] Allegare entrambi i binari (`.exe` per Windows, binario senza estensione per Linux) alla Release
- [ ] Calcolare il **SHA256** del file allegato:
  ```powershell
  Get-FileHash .\GitSearcher.exe -Algorithm SHA256
  ```

## 3. Creare i manifest
- [ ] Installare **winget-create**:
  ```
  winget install Microsoft.WingetCreate
  ```
- [ ] Generare i manifest puntando all'URL della Release:
  ```
  wingetcreate new https://github.com/<user>/GitSearcher/releases/download/v1.0.0/GitSearcher.exe
  ```
  Il tool compila interattivamente i campi e genera i 3 file YAML richiesti.

## 4. Inviare la PR a winget-pkgs
- [ ] Fork di `https://github.com/microsoft/winget-pkgs`
- [ ] Aggiungere i manifest in `manifests/p/Parresia/GitSearcher/1.0.0/`
- [ ] Aprire PR con titolo standard: `Add Parresia.GitSearcher version 1.0.0`
- [ ] Aspettare la validazione automatica (bot + CI); poi la review umana (può richiedere giorni)

## 5. Aggiornamenti futuri
- [ ] Per ogni nuova versione: aggiornare URL + SHA256 e aprire una nuova PR con `wingetcreate update`

---

> **Nota:** se l'app non ha ancora un publisher verificato, il manifest verrà accettato ma il package apparirà come "non verificato" finché non richiedi la verifica del publisher a Microsoft.
