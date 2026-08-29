<div align="center">

<img src=".github/assets/tanren-logo.png" alt="Tanren 鍛錬" width="240">

<h1>Tanren</h1>

<p>
  <b>Allenamento del giapponese: kana, kanji, grammatica.</b>
</p>

<p>
  <img src="https://img.shields.io/badge/Rust-1c1c2e?style=flat-square&logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/Tauri%20v2-1c1c2e?style=flat-square&logo=tauri&logoColor=white" alt="Tauri v2">
  <img src="https://img.shields.io/badge/React-1c1c2e?style=flat-square&logo=react&logoColor=white" alt="React">
  <img src="https://img.shields.io/badge/TypeScript-1c1c2e?style=flat-square&logo=typescript&logoColor=white" alt="TypeScript">
  <img src="https://img.shields.io/badge/SQLite-1c1c2e?style=flat-square&logo=sqlite&logoColor=white" alt="SQLite">
</p>

<p>
  <img src="https://img.shields.io/badge/stato-in%20sviluppo-d94436?style=flat-square" alt="In sviluppo">
  <img src="https://img.shields.io/badge/local--first-nessun%20account-1c1c2e?style=flat-square" alt="Local-first">
</p>

</div>

<br>

<div align="center">

## 鍛 Il nome

I due ideogrammi <b>鍛錬</b> che compongono questa parola rappresentano il concetto di<br>
lavoro costante necessario per raffinare e consolidare un metallo per la sua forgiatura.

Nell'ambito dell'apprendimento di una disciplina fisica, <i>tanren</i> non è riferito tanto<br>
allo sviluppo di un'abilità tecnica specifica, ma piuttosto alla necessaria<br>
preparazione di base preliminare.

<br>

## 錬 Il progetto

<b>Tanren</b> è un'app open-source per l'allenamento del giapponese: hiragana,<br>
katakana, kanji e, in una fase successiva, grammatica.

Ogni argomento si allena in due modi complementari.

<table>
<tr>
<td align="center" width="50%">

### 認識
**Riconoscimento**

Esercizi di matching a scelta multipla, per costruire la lettura immediata.

</td>
<td align="center" width="50%">

### 入力
**Input diretto**

Digitazione della risposta con l'IME giapponese reale del dispositivo.

</td>
</tr>
</table>

<br>

## 稽古 Come funziona

<table>
<tr>
<td align="center" width="33%">

**Local-first**

Nessun account, nessun server. I dati restano sul dispositivo.

</td>
<td align="center" width="33%">

**Ripetizione spaziata**

Un algoritmo SRS decide cosa ripassare, poco prima che venga dimenticato.

</td>
<td align="center" width="33%">

**Mobile-first**

Pensata per lo schermo piccolo e l'uso con il pollice. Anche su desktop.

</td>
</tr>
</table>

</div>

<br>

<div align="center">

## 開発 Sviluppo

</div>

### Prerequisiti

- **Node** 20.19+ oppure 22.12+, come richiede Vite 8.
- **Rust** stabile 1.85+, servito dall'edizione 2024.
- Su Linux, le librerie di sistema di Tauri. Su Ubuntu 24.04:

```bash
sudo apt install -y libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libdbus-1-dev pkg-config
```

Su macOS servono gli strumenti da riga di comando di Xcode, su Windows i Build Tools
di Visual Studio con il carico C++ e WebView2.

### Avviare l'app

```bash
npm install
npm run tauri dev
```

Il primo avvio compila l'intero albero delle dipendenze Rust e richiede qualche
minuto. Quelli successivi sono immediati. La finestra si apre in formato verticale,
perché il design è pensato prima per il telefono.

### Comandi utili

| Comando | Cosa fa |
|---|---|
| `npm run tauri dev` | app completa, shell Tauri più frontend |
| `npm run dev` | solo il frontend nel browser, senza il core Rust |
| `npm run build` | build di produzione del frontend |
| `npm run lint` | oxlint, comprese le regole di confine tra le feature |
| `npm run tauri build` | eseguibile desktop |
| `cargo check --workspace` | compila tutto il lato Rust |
| `cargo test -p tanren-core` | test del dominio, senza avviare Tauri |

<details>
<summary>L'app compila ma crasha all'avvio su Linux</summary>

<br>

Se l'errore assomiglia a questo:

```
symbol lookup error: /snap/core20/.../libpthread.so.0:
undefined symbol: __libc_pthread_init, version GLIBC_PRIVATE
```

il terminale sta ereditando l'ambiente di un editor installato come **snap**, che
inietta percorsi GTK e GIO dentro `/snap`. Il binario finisce per caricare le
librerie sbagliate. Da un terminale di sistema normale non succede. Per lanciarlo
comunque da lì dentro:

```bash
env -u GTK_PATH -u GTK_EXE_PREFIX -u GTK_IM_MODULE_FILE -u GIO_MODULE_DIR \
    -u GSETTINGS_SCHEMA_DIR -u LOCPATH -u XDG_DATA_HOME \
    npm run tauri dev
```

</details>

<br>

<div align="center">

<sub>🚧 Progetto in fase iniziale di sviluppo.</sub>

</div>
