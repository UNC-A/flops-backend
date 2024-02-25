path=""
# ensure absolute dir
cd "$path" || exit
# update software
git pull
cargo update
# compile
cargo b -r
# kill old
killall backend
# run program
nohup cargo r -r > backend_logs &