# Bundled fonts

Vazirmatn is the existing Persian UI font. Additional variable WOFF2 subsets are copied from Fontsource packages version 5.3.0:

- @fontsource-variable/outfit: Latin, weights 100-900.
- @fontsource-variable/lora: Latin, weights 400-700.
- @fontsource-variable/jetbrains-mono: Latin, weights 100-800.
- @fontsource-variable/noto-naskh-arabic: Arabic, weights 400-700.

Each added family includes its original SIL Open Font License in the adjacent family-LICENSE.txt file. No runtime font requests or npm dependencies are needed. Other scripts fall back to the existing font stack.
