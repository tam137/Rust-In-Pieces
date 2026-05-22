# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).


## [V0.4.1] - 2026-05-22

### Added
- Transposition Table (TT) Optimization: Upgraded the legacy simple evaluation cache into a fully fledged Transposition Table (ZobristTable) storing TranspositionEntry containing evaluation, depth, transposition type (Exact, LowerBound, UpperBound), and best move. Added full Alpha-Beta bounds checking and pruning inside the minimax search, along with move ordering enhancements prioritizing the TT best move with PV node rank bonus.

### Fixed



## [V0.4.0] - 2026-05-22

### Added
- Fix divide-by-zero panic in calculate() and coordinate conversion parsing
- Migrated engine to 64-bit Bitboard Architecture (v0.4.0).

### Fixed



## [V0.3.0] - 2026-05-22

### Added
- Refactored engine to single-threaded Iterative Deepening Search, removed Lazy SMP and global_map, fixed board corruption on early search aborts

### Fixed



## [V0.2.7] - 2026-05-21

### Added
- Fix SMP move ordering and add UCI option Threads

### Fixed



## [V0.2.6] - 2026-05-21

### Added
- Added test-specific import of global_map_handler inside move_gen_service tests to silence compiler warnings

### Fixed



## [V0.2.5] - 2026-05-21

### Added
- Fixed E0382 compiler error in stop_flag_termination_test and verified all multithreading unit tests pass

### Fixed



## [V0.2.4] - 2026-05-21

### Added
- Added dynamic compile-time versioning in config.rs and integrated automated changelog workflow

### Fixed



## [V0.2.2] - 2025-02-06

### Added

- postpone evaluation and choose alternate moveordering
- consider 'give check' in evaluation
- fail fast check test in movegen

### Fixed

### Elos

| Name                                | Pkt   | Games | Elo  |
|-------------------------------------|-------|-------|------|
| Rust-In-Pieces V0.2.2 (new)         | 462.5 | 828   | 1573 |
| Rust-In-Pieces V0.2.1               | 427.5 | 828   | 1556 |
| Rust-In-Pieces V0.2.0               | 411.0 | 828   | 1548 |
| Rust-In-Pieces V0.1.4               | 355.0 | 828   | 1521 |


## [V0.2.1] - 2025-01-30

### Added

- improve knight and queen evaluation

### Fixed

### Elos

| Name                               | Pkt   | Games | Elo  |
|------------------------------------|-------|-------|------|
| Rust-In-Pieces V0.2.1 (new)        | 472.5 | 917   | 1550 |
| Rust-In-Pieces V0.2.0              | 464.5 | 917   | 1546 |
| Rust-In-Pieces V0.1.4              | 438.0 | 916   | 1535 |

## [V0.2.0] - 2025-01-14

### Added

- improve knight and queen evaluation

### Fixed

### Elos

| Name                                   | Pkt   | Games | Elo  |
|----------------------------------------|-------|-------|------|
| Rust-In-Pieces V0.2.0-candidate (new)  | 352.0 | 550   | 1564 |
| Rust-In-Pieces V0.1.4                  | 220.0 | 400   | 1520 |
| Rust-In-Pieces V0.1.2                  | 78.5  | 150   | 1509 |
| Rust-In-Pieces V0.1.1                  | 71.5  | 150   | 1491 |
| Rust-In-Pieces V0.1.3-one-thread       | 190.0 | 400   | 1490 |
| Rust-In-Pieces V0.1.3                  | 185.0 | 400   | 1485 |
| Rust-In-Pieces V0.1.0                  | 157.5 | 400   | 1458 |
| SupraH V00i-threaded-3-imp             | 45.5  | 150   | 1421 |


## [V0.1.4] - 2025-01-11

### Added

- remove mutex lock when reading stop_flag and debug_flag to avoid dead locks

### Fixed

- fixed panic when sending 'stop' cmd in 'go infinite' mode introduced by pv node feature

### Elos

| Name                                                | Pkt   | Games | Elo  |
|-----------------------------------------------------|-------|-------|------|
| Rust-In-Pieces V0.1.3-one-thread (new)              | 430.0 | 775   | 1530 |
| Rust-In-Pieces V0.1.4 (new)*                        | 295.5 | 541   | 1520 |
| Rust-In-Pieces V0.1.2                               | 373.5 | 738   | 1518 |
| Rust-In-Pieces V0.1.3                               | 385.0 | 783   | 1516 |
| Rust-In-Pieces V0.1.4-three-threads-candidate (new) | 118.5 | 265   | 1482 |
| Rust-In-Pieces V0.1.1                               | 197.0 | 454   | 1481 |
| Rust-In-Pieces V0.1.0                               |  89.5 | 222   | 1465 |

* all default engines are one-threaded from now


## [V0.1.3] - 2025-01-02

### Added

- implement skip strong validation methods (but disabled)

### Fixed

### Elos

| Name                                | Pkt   | Games | Elo  |
|-------------------------------------|-------|-------|------|
| Rust-In-Pieces V0.1.2               | 423.0 | 703   | 1534 |
| Rust-In-Pieces V0.1.3               | 314.0 | 533   | 1520 |
| Rust-In-Pieces V0.1.1               | 166.0 | 370   | 1466 |
| Rust-In-Pieces V0.1.0               |  84.5 | 204   | 1452 |
| Rust-In-Pieces V0.1.3-candidate     | 165.5 | 496   | 1423 |




## [V0.1.2] - 2024-12-30

### Added

- use block free transposition table (chashmap), reduce cash writing buffer
- use crossbeam-queue

### Fixed

### Elos

| Name                     | Pkt   | Games | Elo  |
|--------------------------|-------|-------|------|
| mewel 0.3.3              |  50.5 |    73 | 1575 |
| Rust-In-Pieces V0.1.2    | 381.5 |   703 | 1513 |
| Rust-In-Pieces V0.1.1    | 344.5 |   706 | 1488 |
| Rust-In-Pieces V0.1.0    | 165.5 |   402 | 1458 |



## [V0.1.1] - 2024-12-24

### Added

- improve overall eval
- eval: knight blocks opponent pawn
- eval: tempo bonus
- store min_max result when depth is only 2

### Fixed


## [V0.1.0] - 2024-12-22

### Added

- technical: refactor code to use threadsave datastructure where needed
- implement multithreading (Lazy SMP)
- implement new thinking time logic
- improve move ordering by PV nodes
- implement asynchronous logger
- implement asynchronous zobrist writer
- implement uci time commands movetime, movestogo and depth
- added some book moves
- bigger improvements in movegenerator (speed)
- solve all compiler warnings

### Fixed

- fixed error when move string was send in uci position command (fix cute chess)
- fixed errors in book moves
- fixed bug when promote to kNight

## [V00i] - 2024-11-22

### Added

- UCI understand debug on/off command
- UCI understand stop command
- UCI understand go infinite command
- improved UCI info strings
- improved ELO in quiescence search (again..) a lot by better cutting (alpha3)

### Fixed

- fixed error when quitting and stdout channel is closed

### Changed

### Removed


## [V00h] - 2024-11-13

### Added

- improved movegen performance
- improved performance in quiescence search
- improved ELO in quiescence search a lot by better cutting
- use cached hashing value instead of doubled eval calculation, improving performance
- added more Book moves
- extended evaluation

### Fixed

### Changed

### Removed

## [V00g] - 2024-11-06

### Added

- added pawn, king, knight and bishop evaluation
- extend logging
- add zobrist hashing

### Fixed

- three move repetition / board hashing

### Changed

### Removed

## [V00f] - 2024-11-05

### Added

- parse UCI time commands and add time management to engine
- Basic Book for move variance
- logging in rust-in-piece.log
- print "info cp" from engine perspective

### Fixed

- improved and fixed UCI protocol move parser when promotion

### Changed

- improved stand pat cuts in Quiescence Search

### Removed

## [V00e] - 2024-11-01

### Added

- Tests, Logging and error-handling

### Fixed

- Fixed critical bug in UCI protocol move parser

### Changed

### Removed

## [V00d] - 2024-10-31

### Added

- Support for en passant
- Better error handling for UCI notation strings

### Fixed

- Fixed bug in promotion notation in the UCI protocol

### Changed

- Refactored move generator

### Removed
