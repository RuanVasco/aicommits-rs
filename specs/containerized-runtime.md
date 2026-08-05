# Spec: container persistente pro runtime do `aic` (inspirado no MCP do GitHub)

Status: ideia capturada, não implementada. Não bloqueia nada do roadmap atual (fases 1-4 de providers).

## Contexto

Hoje `aic` é um binário nativo stateless: cada invocação é um processo curto que lê o repo git local, chama a API do provider configurado e sai. Rodar isso dentro de um container por invocação seria mais burocrático que só ter o binário nativo (precisaria montar o repo como volume e repassar credenciais git toda vez), então isso foi descartado como abordagem de "run" no dia a dia.

O padrão do servidor MCP do GitHub sugere uma alternativa: o client (VS Code/Claude Desktop) sobe o container via `docker run -i --rm ...`, mantém stdio aberto com ele, e o container vive enquanto esse processo pai estiver vivo — quando o client fecha, o pipe cai e o `--rm` derruba o container junto. Não é um container "manualmente gerenciado", é um processo filho de vida curta comandado pelo client.

## Ideia proposta

Ao invés de recompilar `aic` inteiro pra dentro de um container a cada `git commit`, explorar um modelo client/daemon:

- Um modo `aic serve` (ou binário separado) sobe dentro de um container, mantém o provider (chave de API, ou futuramente o modelo Candle carregado em memória — ver fase 4 do roadmap de providers) já inicializado, e escuta em um socket local (named pipe no Windows, unix socket em Linux/WSL) ou porta localhost.
- O binário `aic` do host vira um client fino: em vez de chamar o provider diretamente, ele fala com o daemon local (parsing do diff continua no host, só a chamada de geração de mensagem vai pro daemon).
- Quem sobe/derruba o container é o editor/terminal (uma extensão do VS Code, um task, ou o próprio `aic` na primeira chamada, com um `docker run -d` gerenciado por PID/lockfile) — não o usuário manualmente.

## Por que isso pode valer a pena (e por que talvez não)

**A favor:**
- Se a fase 4 (modelo embedded via Candle) vingar, subir o modelo uma vez e manter em memória evita o custo de carregar o GGUF a cada commit (pode levar segundos).
- Isolamento: toolchain/modelo pesado fica só no container, binário host continua leve.

**Contra / riscos a validar antes de investir:**
- Complexidade de lifecycle (quem garante que o container não fica órfão rodando pra sempre?) é real — o MCP resolve isso porque o client MANTÉM o pipe aberto; replicar isso fora de um client como VS Code exige um mecanismo equivalente (lockfile + healthcheck, por exemplo).
- Ganho só é relevante se o custo de start-up do provider for alto (caso do embedded). Pra Gemini/API remota (estado atual), o daemon não traz benefício nenhum — a latência é toda de rede, não de inicialização local.
- Ainda precisa decidir como o daemon acessa o repo git do usuário (bind mount do diretório de trabalho atual, provavelmente) sem reintroduzir a burocracia que motivou descartar "run" containerizado no primeiro lugar.

## Próximo passo, quando for retomar

Não implementar antes da fase 4 (embedded) estar pronta e validada — é o cenário onde esse desenho realmente paga a conta. Se/quando chegar lá: prototipar o `aic serve` + client mínimo, medir o ganho real de manter o modelo quente vs. recarregar por invocação, e só then decidir se vale a complexidade do lifecycle do container.
