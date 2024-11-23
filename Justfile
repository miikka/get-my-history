release:
    git checkout main
    git show -s
    cargo release patch
