export CC=/home/korniltsev/github/dbudy/cc_hook_compdb/cc.py
./configure
rm cc_hook.txt
make -j1

/home/korniltsev/github/dbudy/cc_hook_compdb/gen_compdb.py /home/korniltsev/github/cpython/cc_hook.txt


# todo make it concurrent
