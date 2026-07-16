:80 {
  encode zstd gzip
  reverse_proxy node-b:8787
}
