# lab-notes

*Read in English: [README.md](README.md)*

Achados medidos nos meus próprios projetos: como as CPUs se comportam de
verdade com cargas reais, e como o meu motor de inferência em CPU se
compara às ferramentas que já existem. Cada achado vem com o protocolo, a
máquina em que rodou e os números brutos, para você conferir ou rodar no
seu próprio hardware.

Por [Fabio Henrique](https://github.com/fabious054) (CodeWave.IT).

## Áreas

| Pasta | O que vai nela |
|---|---|
| [`cpu/`](cpu/) | Comportamento do hardware: clock, caches, núcleos, gerenciamento de energia, armadilhas de medição |
| [`inference/`](inference/) | Resultados do [CandleCLI](https://github.com/fabious054/CandleCLI), motor de inferência de LLM em CPU escrito do zero em Rust puro, medido contra outras ferramentas |

## Achados

| # | Achado | Área | Status |
|---|---|---|---|
| 0001 | [Os núcleos ficam 1,5–1,8× mais lentos por ~10 ms depois de uma fase paralela que termina desigual](cpu/0001-post-attention-slowdown/) | cpu | Uma máquina só, mecanismo confirmado no motor (0004) |
| 0002 | [APERF/MPERF podem ser lidos do modo usuário no Windows pelo RDPRU](cpu/0002-rdpru-user-mode-windows/) | cpu | Uma máquina só |
| 0003 | [Um núcleo que dorme mais de ~1–2 ms volta ~20% mais lento por até 1 ms](cpu/0003-core-wake-ramp/) | cpu | Uma máquina só |
| 0004 | [Leitura rápida de memória derruba o clock do chip inteiro, e a fase seguinte paga por 10–40 ms](cpu/0004-post-streaming-clock-depression/) | cpu | Uma máquina só |

## Como ler um achado

Cada achado é uma pasta com um `README.md` em formato fixo
([TEMPLATE.md](TEMPLATE.md)):

- **Pergunta**: o que se queria saber.
- **Máquina**: CPU, memória, sistema, toolchain.
- **Protocolo**: como foi medido (veja em [PROTOCOL.md](PROTOCOL.md) as
  regras que toda medição segue).
- **Resultados**: os números, com a saída bruta em `data/`.
- **Conclusão**: o que os dados sustentam e o que não sustentam.
- **Status**: `draft`, `single machine`, `replicated by N` ou `refuted`.

Os achados em si são escritos em inglês, para alcançar mais gente.

## O que é (e o que não é) publicado aqui

Achados sobre hardware e o método de medição são publicados por inteiro.
Do motor de inferência, publico os resultados (máquina, modelo, o comando
exato usado na outra ferramenta, números brutos); os detalhes de
implementação do motor não.

## Rode na sua máquina

Se você reproduzir um achado, ou não conseguir, abra uma issue com o
modelo "Replication". Um resultado que discorda vale tanto quanto um que
concorda.

## Licença

[MIT](LICENSE) ([tradução](LICENSE.pt-BR.md)) — código e dados.
