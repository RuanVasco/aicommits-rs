# Spec: provider genérico via template de requisição configurável

Status: ideia capturada, não implementada.

## Contexto

Hoje, suportar uma API nova significa escrever um provider novo em Rust: uma struct pro corpo da requisição, uma struct pra resposta, um `impl CommitProvider`, e uma nova release do `aic`. Mas no fundo toda API de LLM é a mesma coisa — POST de um JSON, resposta em JSON, um texto em algum lugar dentro dela. Isso é dado, não lógica; poderia ser configuração em vez de código.

## Ideia proposta

Um provider genérico (`provider = "custom"`) onde a pessoa (ou um preset já pronto) descreve o formato da requisição, com variáveis sendo injetadas na hora de montar:

```toml
provider = "custom"
url = "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={api_key}"
method = "POST"
# headers vazio nesse caso - a chave do Gemini vai na URL, não em header
[headers]

body = """
{
  "contents": [{ "parts": [{ "text": "{prompt}" }] }],
  "generationConfig": { "maxOutputTokens": 1024, "temperature": 0.2 }
}
"""

response_path = "candidates.0.content.parts.0.text"
```

Ou, pro formato OpenAI-compatible:

```toml
provider = "custom"
url = "{base_url}/chat/completions"
method = "POST"
[headers]
Authorization = "Bearer {api_key}"

body = """
{ "model": "{model}", "messages": [{ "role": "user", "content": "{prompt}" }] }
"""

response_path = "choices.0.message.content"
```

Variáveis disponíveis pra injeção: `{model}`, `{api_key}`, `{prompt}` (o texto já montado do prompt de commit, incluindo o diff). `response_path` é um caminho simples (chave.chave.índice) pra achar o texto gerado dentro do JSON de resposta.

Com isso, Gemini e OpenAI-compatible viram só **presets** desse motor genérico (ou continuam como providers hardcoded, tanto faz pro usuário), e qualquer API nova que apareça — ou uma interna, customizada, sei lá — funciona só preenchendo esse template, sem esperar uma release do `aic`.

## Riscos / pontos a resolver antes de implementar

- **Injeção segura de JSON**: `{prompt}` carrega o diff do usuário, que pode ter aspas, quebras de linha, caracteres unicode — substituição de string crua no meio de um JSON quebra a estrutura. Precisa montar o body como valor JSON de verdade (via `serde_json`) e escapar a substituição corretamente, não fazer find/replace ingênuo na string.
- **`response_path`**: precisa de uma implementação mínima mas correta de navegação por chave/índice no JSON de resposta. Não precisa ser um JSONPath completo, mas precisa cobrir os casos reais (arrays aninhados, chave ausente, null) com testes pra cada formato conhecido.
- **UX do wizard**: pedir pra pessoa digitar um template multi-linha + mapa de headers via prompts do `dialoguer` no terminal é bem mais complicado que os campos de texto simples de hoje. Pode ser mais realista deixar o preset "Custom" ser editado direto no `config.toml` (com o formato documentado no README) em vez de tentar montar um editor interativo no terminal.
- Decidir se vale a pena migrar Gemini/OpenAI-compatible pra rodar em cima desse motor genérico também (unifica o código, mas é risco de regressão em algo que já funciona) ou só adicionar "Custom" como uma terceira opção nova, mantendo os dois providers atuais como estão.

## Próximo passo, quando for retomar

Prototipar o motor de template reproduzindo os dois formatos já conhecidos (Gemini e OpenAI-compatible) e comparar byte a byte com o que os providers hardcoded produzem hoje, antes de expor "Custom" como opção no wizard de setup.
