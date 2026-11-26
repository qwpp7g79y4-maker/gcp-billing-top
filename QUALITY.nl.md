# Kwaliteit

pep: 10

Geteste codecommit: a3d92b5 (gemeten 2026-11-25; de commit erna wijzigt alleen documenten).
Engelse versie: [QUALITY.md](QUALITY.md). De koppeling van beweringen aan
tests staat daar; de testnamen zijn Engels.

Tests: 244

Het getal is het aantal testfuncties in `src/` en `tests/`. De test
`quality_test_count_equals_suite` controleert QUALITY.md daarop.

| Set | Waar | Tests |
| --- | --- | --- |
| unittests | `src/*.rs` | 158 |
| end-to-end (echt programma, testbestanden) | `tests/cli.rs` | 72 |
| documenten, licenties, manifest | `tests/docs.rs` | 14 |

## Poorten

Draai vanuit de hoofdmap van de repository:

```text
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo publish --dry-run
```

De gemeten uitslag van deze poorten staat in het volgende kopje. De kopjes
daarna zijn eerdere metingen, ongewijzigd bewaard.

## Gemeten 2026-11-25, fix a3d92b5 (clippy op macOS en Windows)

Datum: 2026-11-25. Fixcommit: a3d92b5. Machine: Linux 7.2.15. Schone worktree van a3d92b5.

Gat: CI-run 45698473602 faalde op clippy, niet op de tests, op macos-latest,
macos-15-intel en windows-latest: `tests/cli.rs:925`, `function
run_with_stdout_redirect is never used`. De twee tests die haar aanroepen staan
onder `cfg(target_os = "linux")`; de hulpfunctie niet. Die staat nu onder
dezelfde `cfg`, zonder `allow(dead_code)` en zonder `expect`. De twee
Linux-tests zijn ongewijzigd (exit 2 bij gesloten standaarduitvoer, exit 0 bij
`/dev/null`).

| Opdracht | Uitkomst |
| --- | --- |
| `cargo fmt --check` | exit 0 |
| `cargo clippy --all-targets --locked -- -D warnings` | exit 0, geen waarschuwingen |
| `cargo test --locked` | exit 0: 158 unittests, 0 in `src/main.rs`, 72 in `tests/cli.rs`, 14 in `tests/docs.rs`, 0 doctests: 244 geslaagd, 0 mislukt |
| `cargo test --release --locked` | exit 0: dezelfde 244 geslaagd, 0 mislukt |
| `cargo +1.85 fmt --check` | exit 0 (rustc 1.85.1, gemeten 2026-11-25) |
| `cargo +1.85 clippy --all-targets --locked -- -D warnings` | exit 0, geen waarschuwingen (rustc 1.85.1, gemeten 2026-11-25) |
| `cargo +1.85 test --locked` | exit 0: 158 unittests, 0 in `src/main.rs`, 72 in `tests/cli.rs`, 14 in `tests/docs.rs`, 0 doctests: 244 geslaagd, 0 mislukt (rustc 1.85.1, gemeten 2026-11-25) |

De 1.85-rijen zijn de gedeclareerde `rust-version`, gedraaid op de ongewijzigde
bronnen; `cargo +1.85 test --release --locked` exit 0: 158 unittests, 0 in `src/main.rs`, 72 in `tests/cli.rs`, 14 in `tests/docs.rs`, 0 doctests: 244 geslaagd, 0 mislukt (rustc 1.85.1, gemeten 2026-11-25, aparte doelmap).

Niet uitgevoerd (not run): clippy en tests op macos-latest, macos-15-intel en
windows-latest (de CI-run hierboven is de falende, er is geen nieuwe run),
`cargo deny`, `cargo audit`, `cargo publish --dry-run`, fuzzing, mutation
testing. Er is niets gepusht.

## Gemeten 2026-11-25, fix a7b3b5c (gesloten standaarduitvoer)

Datum: 2026-11-25. Fixcommit: a7b3b5c. Machine: Linux 7.2.15. Schone worktree van 3bfff1c plus de fix.

Gat: met standaarduitvoer gesloten bij de start (`>&-`) stopte het programma
met 0 en schreef niets, omdat Rust een gesloten descriptor 1 vóór `main` opnieuw
opent als `/dev/null`. Nu volgt afsluitcode 2 met `standard output is closed`;
`>/dev/null` blijft 0. De controle is een constructor in `.init_array`
(`src/stdout_probe.rs`, de enige `unsafe` in het programma), alleen Linux.

Rood, vóór de fix, `cargo test --locked --test cli standard_output`:
1 geslaagd (`/dev/null`), 1 mislukt (`closed_standard_output_at_start_is_exit_2_with_a_message`, links 0, rechts 2).

| Opdracht | Uitkomst |
| --- | --- |
| `cargo fmt --check` | exit 0 |
| `cargo clippy --all-targets --locked -- -D warnings` | exit 0, geen waarschuwingen |
| `cargo test --locked` | exit 0: 158 unittests, 0 in `src/main.rs`, 72 in `tests/cli.rs`, 14 in `tests/docs.rs`, 0 doctests: 244 geslaagd, 0 mislukt |
| `cargo test --release --locked` | exit 0: dezelfde 244 geslaagd, 0 mislukt |

Gecontroleerd en geen gat: er is geen `--output`-optie en geen `build.rs`, dus
geen schrijven via symlink of naar een speciaal bestand en geen lezen van
`.git/HEAD`. Niet uitgevoerd: `cargo deny`, `cargo audit`,
`cargo publish --dry-run`, een GitHub Actions-run, een Windows- of macOS-run
(daar doet de controle niets), fuzzing, mutation testing.

## Gemeten 2026-11-25, commit c837bdb

Datum: 2026-11-25. Commit: c837bdb. Machine: Linux 7.2.15, cargo 1.99.0, rustc 1.99.0.
Een schone worktree van c837bdb, niets in de bron gewijzigd.

| Opdracht | Uitslag |
| --- | --- |
| `cargo fmt --check` | exit 0 |
| `cargo clippy --all-targets --locked -- -D warnings` | exit 0, geen waarschuwingen |
| `cargo test --locked` | exit 0: 158 unittests, 0 in `src/main.rs`, 70 in `tests/cli.rs`, 14 in `tests/docs.rs`, 0 doctests: 242 geslaagd, 0 mislukt |
| `cargo test --release --locked` | exit 0: dezelfde 242 geslaagd, 0 mislukt |

Niet uitgevoerd (not run) in deze meting: `cargo deny`, `cargo audit`,
`cargo publish --dry-run`, een GitHub Actions-run, een Windows- of macOS-run,
fuzzing, mutatietests.

## Gemeten 2026-11-24, review gcp-eind-ad79106 (B-1)

Commando: `cargo test --locked --offline --no-fail-fast`, uitgevoerd op 2026-11-24.

Rood, op ad79106 met de nieuwe tests erbij gekopieerd en de bron niet hersteld:
de cargo-regel was `test result: FAILED. 68 passed; 2 failed` voor
`tests/cli.rs` (de twee projectkolomtests) en
`test result: FAILED. 12 passed; 2 failed` voor `tests/docs.rs` (de twee
telltests).

Groen, op de nieuwe tip (codecommit 7f1a4d8 plus de documentcommit): 158
unittests, 0 in `src/main.rs`, 70 in `tests/cli.rs`, 14 in `tests/docs.rs`, 0
doctests: 242 geslaagd, 0 mislukt. De commit na 7f1a4d8 wijzigt alleen
documenten. De eerste commit, 7f1a4d8, is zelf rood op de twee telltests,
omdat de getallen in documenten staan.

Dezelfde projectnaam twee keer (`project.id` en `Project ID`, of `project.name`
en `Project name`) geeft nu afsluitcode 2. `project.id` naast `project.name`
blijft geldig en `project.id` wint. De weigering van `usage_start_time` samen
met `invoice.month` is ongewijzigd; er is geen maandkolomoptie.

Niet uitgevoerd: een GitHub Actions-run (er is niets gepusht). Open en
ongewijzigd: bevindingen 1 tot en met 11 van de review gcp-eind-ad79106.

STAND: groen

## Gemeten 2026-11-24, review gcp-eind-4bd85a5 (B-1 en B-2)

Opdracht: `cargo test --locked --offline --no-fail-fast`, op 2026-11-24 gedraaid.

Rood, op 4bd85a5 met de nieuwe tests erbij gekopieerd en de bron niet hersteld:
de cargo-regel was `test result: FAILED. 60 passed; 8 failed` voor `tests/cli.rs`
(de zes kolomtests en de twee tests met lege regels; de test zonder lege regels
slaagde, zoals hoort) en `test result: FAILED. 12 passed; 2 failed` voor
`tests/docs.rs` (de twee telltests).

Groen, op de nieuwe tip (codecommit b4b3011 plus de documentcommit): 158 unittests, 0 in
`src/main.rs`, 68 in `tests/cli.rs`, 14 in `tests/docs.rs`, 0 doctests: 240
geslaagd, 0 mislukt. De commit na b4b3011 wijzigt alleen documenten. De eerste
commit, b4b3011, is zelf rood op de twee telltests, omdat de getallen in
documenten staan.

Bewust gewijzigd: de unittest `usage_start_beats_invoice_month` is vervangen door
`usage_start_with_invoice_month_is_ambiguous`, want de voorrang die hij toetste
is de stille keuze die B-1 verbiedt. `project.id` naast `project.name` wordt niet
geweigerd (een BigQuery-export heeft beide); de projectkolom hoort niet bij B-1.

Niet uitgevoerd: een GitHub Actions-run (niets gepusht). Open en ongewijzigd: bevindingen 1 tot en met 14
van de review gcp-eind-4bd85a5.

STAND: groen

## Gemeten 2026-11-24, review gcp-eind-bc4169c (B-1 tot en met B-5)

Opdracht: `cargo test --locked --no-fail-fast`, op 2026-11-24 gedraaid.

Rood, op bc4169c met de nieuwe `tests/cli.rs` en `tests/docs.rs` erbij gekopieerd
en de bron niet hersteld (B-1 tot en met B-5 van review gcp-eind-bc4169c): de
cargo-regel was `test result: FAILED. 46 passed; 13 failed` voor
`tests/cli.rs` en `test result: FAILED. 11 passed; 3 failed` voor
`tests/docs.rs` (de twee telltests en de `.gitattributes`-test). De 13
mislukte end-to-endtests zijn de CRLF-regelnummers, de overlopende sommen, de
kostenkolommen, de herhaalde sleutels en de valutacode. De nieuwe unittests in
`src/` compileren niet op de oude bron, dus ze zitten niet in het rode getal.

Groen, op de nieuwe tip (codecommit 9580598 plus de documentcommit): 158
unittests, 0 in `src/main.rs`, 59 in `tests/cli.rs`, 14 in `tests/docs.rs`, 0
doctests: 231 geslaagd, 0 mislukt. De commit na 9580598 wijzigt alleen
documenten. De eerste commit, 9580598, is zelf rood op de twee telltests
(`readme_test_count_equals_suite`, `quality_test_count_equals_suite`), omdat de
getallen in documenten staan.

STAND: groen

## Niet uitgevoerd

Op 2026-11-24 zijn `cargo fmt --check` en
`cargo clippy --all-targets --locked -- -D warnings` ook met exit 0 geëindigd
op 9580598, en `cargo test --release --locked` slaagde voor dezelfde 231 tests. Niet uitgevoerd: `cargo deny`,
`cargo audit`, `cargo publish --dry-run`, een GitHub Actions-run (het
CRLF-probleem is hersteld in de bron en met `.gitattributes`, maar er is geen
run op GitHub of Windows geweest), fuzzing en mutatietests. Open en
ongewijzigd: bevindingen a, b en c van de review (a: de ingebouwde
tarieven in QUALITY.md, die in de bron niet bestaan; b: geheugengebruik;
c: het bereik van de jaren), en de overige open bevindingen van die review.

Deze pagina legt controles vast. Het is geen garantie en geen fuzzrapport.
