:80 {
  encode zstd gzip
  reverse_proxy node-c:8787
}
