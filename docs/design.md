# Design

Core flow:

1. parse and validate manifest
2. resolve artifacts and required checks
3. collect candidate files
4. create tar.gz archive
5. compute sha256 and metadata
6. upload via ossutil and append records
