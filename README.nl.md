# gcp-billing-top

Een klein opdrachtregelprogramma dat een **GCP billing-export CSV** leest en
laat zien waar het geld heen gaat: de hoogste kosten per **SKU**, **project**
en **label**, met de verandering ten opzichte van de maand ervoor.

De tests en de gemeten aantallen staan in `QUALITY.md` (Nederlands:
[QUALITY.nl.md](QUALITY.nl.md)). De Engelse versie van deze pagina is
[README.md](README.md).

Het werkt op een bestand dat je al hebt. Het praat niet met BigQuery, niet met
het netwerk en niet met een andere dienst.

## Aan de slag in 5 minuten

1. Installeer het (zie Installatie hieronder).
2. Neem een billing-export CSV die je al hebt.
3. Draai `gcp-billing-top export.csv`.
4. Voeg `--by sku` of `--by project` toe voor één lijst.
5. Voeg `--sort delta` toe om de grootste veranderingen eerst te zien.

Zonder eigen export kun je een testbestand uit deze repository gebruiken:

```text
cargo build --release
target/release/gcp-billing-top tests/fixtures/two_months.csv
```

Het rapport gaat naar standaarduitvoer; het programma schrijft geen bestanden.

## Gebruik

```text
gcp-billing-top export.csv                      # laatste maand, verschil met de maand ervoor
gcp-billing-top october.csv -p september.csv    # vorige maand in een tweede bestand
gcp-billing-top export.csv --by sku -n 20       # alleen de SKU-lijst, 20 regels
gcp-billing-top export.csv --sort delta         # grootste veranderingen eerst
gcp-billing-top export.csv --by label --label-key env
gcp-billing-top export.csv --net -f json        # na aftrek van tegoeden, als JSON
cat export.csv | gcp-billing-top -              # standaardinvoer
```

## Voorbeeld

Invoer `tests/fixtures/two_months.csv`, optie `--by sku`:

```text
gcp-billing-top: 2026-10 vs 2026-09 (gross, USD)
Total: 300.00  (previous 250.00, +50.00, +20.0%)

Top 5 by SKU
  #  Name                                    Cost  Previous   Delta  Delta %
  1  N2 Instance Core running in Americas  200.00    150.00  +50.00   +33.3%
  2  Standard Storage US Multi-region       60.00     50.00  +10.00   +20.0%
  3  BigQuery Analysis                      25.00      0.00  +25.00      new
  4  Network Egress                         10.00     30.00  -20.00   -66.7%
  5  Support                                 5.00      0.00   +5.00      new
```

## Wat het doet

Elke bewering hieronder is door een test gedekt; [QUALITY.nl.md](QUALITY.nl.md)
zegt welke.

- **Alleen offline.** Het leest een CSV-bestand of standaardinvoer. Er is geen
  afhankelijkheid van netwerk of BigQuery.
- **Hoogste kosten.** Gerangschikte lijsten per SKU, per project en per label
  (`--by sku|project|label`, standaard: elke lijst waarvoor het bestand kolommen
  heeft).
- **Maandverschil in één export.** De laatste maand (of `--month`) wordt
  vergeleken met de kalendermaand ervoor, als de export die bevat.
- **Maandverschil uit een tweede bestand.** `--previous FILE` haalt de vorige
  maand uit een ander bestand; de uitkomst is gelijk aan het rapport met één
  bestand.
- **Beide exportvormen.** BigQuery-namen en Cloud Console-namen; datums als
  ISO-tijdstempel, `YYYYMM` of `Oct 1, 2026`; een byte-order mark wordt
  genegeerd. De kostenkolom is `cost` of `Cost (` met een munteenheid of teken
  en `)`, zoals `Cost (EUR)`, en niets anders: `cost_at_list`, `cost center` en
  `Unrounded cost (EUR)` zijn andere kolommen. Twee kolommen
  van één soort (kosten, maand, credits, labels, SKU of valuta) worden
  geweigerd met afsluitcode 2; er wordt er geen gekozen op positie of op volgorde
  van namen. Dat geldt ook voor `usage_start_time` samen met `invoice.month`,
  ook als beide dezelfde maand noemen. Dezelfde projectnaam twee keer
  (`project.id` met `Project ID`, of `project.name` met `Project name`) wordt
  ook geweigerd. (`project.id` naast `project.name` is gewoon en houdt
  `project.id`.)
- **Labels.** BigQuery-JSON, een JSON-object of `env=prod;team=web`. Zonder
  `--label-key` telt een regel mee voor elk van zijn labels, dus labelkosten
  kunnen boven het totaal uitkomen. Met `--label-key env` wordt gegroepeerd op
  de waarde van `env` en tellen de groepen op tot het totaal. Een labelsleutel
  die twee keer in één regel staat, in JSON of in `env=prod;env=dev`, wordt
  geweigerd met afsluitcode 2: geen van beide waarden wordt gekozen.
- **Tegoeden.** `--net` telt de tegoedkolom bij de kosten op, in beide maanden.
  Een tegoed met de sleutel `"amount"` twee keer wordt geweigerd met
  afsluitcode 2.
- **Exacte sommen.** Bedragen worden als miljoenste ingelezen en als gehele
  getallen opgeteld, dus totalen lopen niet uit. Wetenschappelijke notatie
  als `1.5e-3` wordt geaccepteerd. Bereik: één bedrag moet kleiner zijn dan
  1e12; een som (een totaal, een groep, kosten plus tegoeden, een verschil
  tussen maanden) moet binnen -9223372036854.775808 tot 9223372036854.775807
  blijven, na elke regel gecontroleerd in bestandsvolgorde. Een som buiten dat
  bereik wordt geweigerd met een fout dat hij te groot is; hij wordt nooit op
  de grens vastgezet.
- **Drie formaten.** `-f table` (standaard), `-f json`, `-f csv`. In de tabel worden besturingstekens in namen
  (regelwisseling, tab, escape) als `?` getoond. In CSV krijgt een naam die met
  `=`, `+` of `@` begint een `'` ervoor, zodat een spreadsheet hem als tekst leest.
- **Eén munteenheid tegelijk.** Mengen de vergeleken maanden munteenheden, dan
  stopt het en vraagt om `--currency CODE`.
- **Duidelijke fouten.** Problemen noemen bestand en regel; het programma telt
  de regels zelf, dus een bestand met CRLF krijgt dezelfde regel als hetzelfde
  bestand met LF. Afsluitcode 1 voor datafouten, 2 voor verkeerde opties en
  voor invoer die twee dingen zegt (zie de bewering over kolommen, labels en
  tegoeden). Bij een fout gaat niets naar standaarduitvoer. Een valutacode uit
  het bestand toont besturingstekens als `?` in de foutmelding, net als namen
  in de tabel.
- **Standaardinvoer.** `-` leest het hoofdbestand (of het `--previous`-bestand)
  van stdin.
- **Grenzen en volgorde.** `-n N` regels per lijst (standaard 10, `0` = alles);
  `--sort delta` rangschikt op grootte van de verandering en toont ook groepen
  die verdwenen zijn.
- **Eén maand is goed.** Een bestand met één maand geeft nog steeds de
  hoogste-kostenlijsten en meldt dat er geen verschil is.

## Opties

| Optie | Betekenis |
| --- | --- |
| `FILE` | export-CSV, of `-` voor stdin |
| `-p`, `--previous FILE` | tweede bestand met de vorige maand |
| `--month YYYY-MM` | te rapporteren maand (standaard: laatste) |
| `--previous-month YYYY-MM` | maand om mee te vergelijken |
| `-b`, `--by sku\|project\|label` | welke lijsten (herhaalbaar) |
| `--label-key KEY` | groepeer de labellijst op de waarde van KEY |
| `-n`, `--top N` | regels per lijst, `0` = alles |
| `--sort cost\|delta` | volgorde |
| `--net` | tegoeden aftrekken |
| `--currency CODE` | alleen regels in deze munteenheid |
| `-f`, `--format table\|json\|csv` | uitvoerformaat |

Regels zonder SKU, project of labels staan onder `(no sku)`, `(no project)` en
`(no labels)`.

## Installatie

Versie 0.1.0. Van crates.io, zodra 0.1.0 daar staat:

```sh
cargo install gcp-billing-top
```

Of bouw uit de broncode.

```text
cargo install --path .
cargo test
```

## Afsluitcodes

| Code | Betekenis |
| --- | --- |
| 0 | het rapport is geschreven |
| 1 | een datafout: onleesbaar bestand, foutief bedrag of label, een som die te groot is, gemengde munteenheden, ontbrekende maand |
| 2 | verkeerde opties (gemeld door de argumentenlezer), of dubbelzinnige invoer: twee kostenkolommen, een herhaalde `amount`- of labelsleutel |

Bij een fout gaat niets naar standaarduitvoer; de melding gaat naar
standaardfout en noemt bestand en regel.

## Beperkingen

- Het leest één CSV-export (plus eventueel een tweede bestand voor de vorige
  maand). Het haalt geen gegevens op uit BigQuery of van elders.
- Het vergelijkt alleen kalendermaanden.
- Het werkt met één munteenheid tegelijk; gemengde munteenheden stoppen de run
  tot `--currency CODE` er één kiest.
- Zonder `--label-key` telt een regel mee voor elk van zijn labels, dus
  labelkosten kunnen samen meer zijn dan het totaal.
- Het toont de kosten uit het bestand zoals ze zijn. Het vergelijkt ze niet met
  een factuur.

## Onderhoud

Wijzigingen lopen via Issues, zie [CONTRIBUTING.md](CONTRIBUTING.md).
Beveiligingsproblemen meld je zoals in [SECURITY.md](SECURITY.md) staat. Elke
gedragswijziging komt met een test, en het aantal tests in dit bestand en in
`QUALITY.md` wordt door de testset gecontroleerd.

## Kwaliteit

Tests: 244

Details en de koppeling van beweringen aan tests staan in
[QUALITY.nl.md](QUALITY.nl.md).

## Licentie

Beschikbaar onder een van beide: [Apache License, Version 2.0](LICENSE-APACHE)
of de [MIT-licentie](LICENSE-MIT), naar keuze.

De licenties van de externe crates in het programma staan in [THIRD-PARTY-LICENSES.html](THIRD-PARTY-LICENSES.html).
