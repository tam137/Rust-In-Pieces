# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).


## [V00j] - 2024-12-XX

### Added

- bigger improvments in movegenerator (about 50% faster)

### Fixed

- fixed error when mo move string was send in uci position command (fix cute chess)
- fixed errors in book moves

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
