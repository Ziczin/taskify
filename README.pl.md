# Taskify

[English](README.en.md) · [Русский](README.ru.md) · [日本語](README.ja.md) · [Українська](README.uk.md) · **Polski** · [Español](README.es.md) · [中文](README.zh.md) · [Deutsch](README.de.md)

Taskify to lokalna aplikacja do grywalizacji osobistych zadań na Windows. Prowadź nawyki, zadania powtarzalne i jednorazowe oraz listy kontrolne; zdobywaj XP i monety; odblokowuj osiągnięcia, motywy, kosmetyki i naklejki. Wszystko jest przechowywane na komputerze w dzienniku zdarzeń JSONL — bez konta i bez sieci.

![Ekran „Dzisiaj”](docs/presentation/today.png)

## Możliwości

- Cztery rodzaje zadań: nawyki, zadania powtarzalne, jednorazowe i listy kontrolne z podzadaniami.
- Poziomy bez limitu, postęp dzienny i tygodniowy, serie, kombinacje i osiągnięcia.
- Monety, sklep z kosmetykami, motywy, akcenty, tła, czcionki, dźwięki i przeciągane naklejki.
- Pakiety treści w TOML: zadania, kategorie, tagi, ikony, palety, kombinacje, osiągnięcia, kosmetyki i zestawy.
- Kilka przestrzeni (livespace): każda ma własną historię, zakupy i tablicę naklejek.
- Osiem języków interfejsu: rosyjski, angielski, japoński, ukraiński, polski, hiszpański, chiński i niemiecki.
- Wyłącznie lokalne przechowywanie: dziennik zdarzeń, przebudowywalne pamięci podręczne i automatyczne kopie zapasowe.
- Tryb przenośny przez `TASKIFY_HOME`.

## Wymagania

- Windows 10 lub nowszy.
- Rust 1.85 lub nowszy.
- Narzędzia kompilacji C++, jeśli nie ma ich w zainstalowanym Rust.

## Budowanie i uruchamianie

```powershell
cargo build --release
.\target\release\taskify.exe
```

Ten sam plik udostępnia polecenia wiersza poleceń:

```powershell
.\target\release\taskify.exe version
.\target\release\taskify.exe packs
.\target\release\taskify.exe stats default
.\target\release\taskify.exe validate history default
```

Przy pierwszym uruchomieniu tworzona jest przestrzeń `default` i instalowane są wbudowane pakiety.

## Dane

Domyślnie wszystko znajduje się w `%APPDATA%\taskify`:

- `livespaces\<nazwa>` — dziennik `data/log/YYYY-MM.jsonl`, pamięci podręczne `state.json` i `aggregates.json`, kopie zapasowe.
- `packs\<id>` — zainstalowane pakiety treści.
- `settings.toml` — ustawienia okna i aplikacji.

Zmienna `TASKIFY_HOME` przenosi cały katalog danych w inne miejsce (tryb przenośny). Źródłem prawdy jest dziennik: pamięci podręczne można usunąć i odtworzyć ze zdarzeń.

## Wiersz poleceń

```text
taskify version
taskify packs [<data-root>]
taskify validate pack <ścieżka-do-pakietu>
taskify validate all [<data-root>]
taskify validate history <livespace>
taskify stats <livespace>
taskify log <livespace> [--limit N]
taskify rebuild <livespace>
taskify backup <livespace>
taskify import <archiwum.zip> <livespace> [<data-root>]
taskify export <livespace> <archiwum.zip>
```

## Struktura projektu

| Crate | Odpowiedzialność |
| --- | --- |
| `crates/core` | Zadania, nagrody, krzywa poziomów, metryki, granice dnia. |
| `crates/store` | Zdarzenia, dziennik, pamięci podręczne, odtwarzanie, kopie zapasowe, przestrzenie. |
| `crates/packs` | Ładowanie pakietów TOML, rejestry, walidator, instalacja zip. |
| `crates/ui` | Okno egui/eframe, ekrany, zasady gry, lokalizacja, CLI. |


## Pakiety

Pakiet to folder z `pack.toml` i podfolderami zadań, kombinacji, osiągnięć, kosmetyków, zestawów, definicji i multimediów. Identyfikatory otrzymują prefiks pakietu (`starter/water`). Wbudowane pakiety znajdują się w `assets/packs` i są instalowane przy pierwszym uruchomieniu.

## Testy

```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
.\target\release\taskify.exe validate all
```

Obecny zestaw ma **155 testów**.

## Współpraca

Issues i pull requesty są mile widziane. Przed pull requestem uruchom powyższe testy i przetłumacz wszystkie teksty interfejsu na każdy język z `crates/ui/src/i18n.rs`.

## Licencja

Taskify jest udostępniany na licencji MIT. Zobacz [LICENSE](LICENSE). Dołączone czcionki Noto są objęte SIL Open Font License; zobacz `assets/fonts/LICENSE-OFL.txt`.
