release:
    git checkout -q main
    git show -s
    cargo release patch --execute
