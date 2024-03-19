# deleted modified local cache
#
# due to issues with file ownership this must be performed
git checkout . >> backend_logs

# update software
git pull
cargo update


# compile
mold --run cargo b -r >> backend_logs