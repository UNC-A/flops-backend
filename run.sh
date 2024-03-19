# deleted modified local cache
#
# due to issues with file ownership this must be performed
git checkout .
# compile
mold --run cargo b -r
# run program
nohup cargo r -r > backend_logs &