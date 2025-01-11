# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).


## TODO

Technical: remove Mutex from PV-Nodes in
Movegen: add null move threads
Movegen: improve Sorting of capture moves
Eval: detect and evaluate pawn islands
Eval: advance rook game


## [V0.2.0] - 2025-01-XXs

### Added

### Fixed

### Elos

| Name                                                | Pkt   | Games | Elo  |
|-----------------------------------------------------|-------|-------|------|


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
