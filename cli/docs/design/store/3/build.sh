set -eu

mkdir src
cd src
tar -xf $src/*
$src/configure --prefix=$o
make
make install DESTDIR=$t
