# Niet van toepassing

Elke regel is `stuk: reden`. Een stuk staat hier alleen als het niet in de repository bestaat.

build.rs: het pakket genereert niets bij het bouwen en heeft geen buildstap nodig
man: er is geen man-pagina; `--help` en de README beschrijven elke optie
docs: alle documentatie staat in README, QUALITY, CONTRIBUTING en SECURITY, er is geen aparte map
examples: het voorbeeld staat in de README en wordt door een test met de echte uitvoer vergeleken
samples: de CSV-voorbeelden staan als testbestanden in tests/fixtures
scripts: er zijn geen hulpscripts; de poorten zijn gewone cargo-opdrachten in QUALITY.md
.github/workflows/release.yml: er is nog geen release of publicatie; alleen ci.yml bestaat
