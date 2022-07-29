# Export all env variables in .env file.
# https://stackoverflow.com/questions/43267413/shell-how-to-set-environment-variables-from-env-file
export $(xargs <.env)

cargo run --release -- -s visual-members