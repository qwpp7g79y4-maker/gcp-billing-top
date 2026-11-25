# Kwaliteit

Geteste codecommit: 7f1a4d8 (de laatste commit die geen puur document is).
Engelse versie: [QUALITY.md](QUALITY.md). De koppeling van beweringen aan
tests staat daar; de testnamen zijn Engels.

Tests: 242

Het getal is het aantal testfuncties in `src/` en `tests/`. De test
`quality_test_count_equals_suite` controleert QUALITY.md daarop.

| Set | Waar | Tests |
| --- | --- | --- |
| unittests | `src/*.rs` | 158 |
| end-to-end (echt programma, testbestanden) | `tests/cli.rs` | 70 |
| documenten, licenties, manifest | `tests/docs.rs` | 14 |

## Poorten

Draai vanuit de hoofdmap van de repository:

```text
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo publish --dry-run
```

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
ongewijzigd: N1 tot en met N11 van de review gcp-eind-ad79106.

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

Niet uitgevoerd: een GitHub Actions-run (niets gepusht). Open en ongewijzigd: N1 tot en met N14
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
ongewijzigd: bevindingen N-a, N-b en N-c van de review (N-a: de ingebouwde
tarieven in QUALITY.md, die in de bron niet bestaan; N-b: geheugengebruik;
N-c: het bereik van de jaren), en de overige N-bevindingen.

Deze pagina legt controles vast. Het is geen garantie en geen fuzzrapport.
