# update software
git pull
cargo update
# compile
mold --run cargo b -r
# run program
nohup cargo r -r > backend_logs &