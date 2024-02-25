path=""
# ensure absolute dir
cd "$path" || exit
# update software
git pull
cargo update
# compile
cargo b -r
# run program
nohup cargo r -r > backend_logs &