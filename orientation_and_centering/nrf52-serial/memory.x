MEMORY
{
  /* NOTE 1 K = 1 KiBi = 1024 bytes */
  /* You must fill in these values for your application */
  FLASH : ORIGIN = 0x00000000 + 112K, LENGTH = 1024K - 112K
  RAM : ORIGIN = 0x20000000 + 10K, LENGTH = 256K - 10K
}
