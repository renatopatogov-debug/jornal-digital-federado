use std::env;
use std::io;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::time::timeout;

const REDIS_IO_TIMEOUT: Duration = Duration::from_millis(250);
const MAX_BULK_BYTES: usize = 4 * 1024 * 1024;
const FEED_EPOCH_KEY: &str = "jornal:feed:epoch";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RedisEndpoint {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub db: Option<u32>,
}

#[derive(Debug, PartialEq, Eq)]
enum RespValue {
    SimpleString(String),
    Integer(i64),
    BulkBytes(Option<Vec<u8>>),
    Error(String),
}

#[derive(Clone)]
pub struct RedisService {
    endpoint: Option<Arc<RedisEndpoint>>,
    conn: Arc<Mutex<Option<BufReader<TcpStream>>>>,
}

static GLOBAL_REDIS: OnceLock<RedisService> = OnceLock::new();

impl RedisService {
    pub fn disabled() -> Self {
        Self {
            endpoint: None,
            conn: Arc::new(Mutex::new(None)),
        }
    }

    pub fn from_url(raw_url: &str) -> Self {
        let endpoint = parse_redis_url(raw_url).map(Arc::new);
        Self {
            endpoint,
            conn: Arc::new(Mutex::new(None)),
        }
    }

    pub fn global() -> &'static Self {
        GLOBAL_REDIS.get_or_init(|| {
            let url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
            Self::from_url(&url)
        })
    }

    pub fn is_configured(&self) -> bool {
        self.endpoint.is_some()
    }

    pub async fn ping(&self) -> bool {
        matches!(
            self.executar_comando(&[b"PING"]).await,
            Some(RespValue::SimpleString(ref s)) if s.eq_ignore_ascii_case("PONG")
        )
    }

    pub async fn get_string(&self, key: &str) -> Option<String> {
        match self.executar_comando(&[b"GET", key.as_bytes()]).await {
            Some(RespValue::BulkBytes(Some(bytes))) => String::from_utf8(bytes).ok(),
            _ => None,
        }
    }

    pub async fn set_ex(&self, key: &str, ttl_secs: u64, value: &str) -> bool {
        let ttl_str = ttl_secs.max(1).to_string();
        matches!(
            self.executar_comando(&[
                b"SETEX",
                key.as_bytes(),
                ttl_str.as_bytes(),
                value.as_bytes(),
            ])
            .await,
            Some(RespValue::SimpleString(ref s)) if s.eq_ignore_ascii_case("OK")
        )
    }

    /// Incrementa contador atômico de janela fixa no Redis 7, aplicando TTL na abertura da janela
    pub async fn incr_window(&self, key: &str, ttl_secs: u64) -> Option<u64> {
        let count = match self.executar_comando(&[b"INCR", key.as_bytes()]).await {
            Some(RespValue::Integer(v)) if v >= 1 => v as u64,
            _ => return None,
        };

        if count == 1 {
            let ttl_str = ttl_secs.max(1).to_string();
            let _ = self
                .executar_comando(&[b"EXPIRE", key.as_bytes(), ttl_str.as_bytes()])
                .await;
        }

        Some(count)
    }

    /// Obtém a geração atual do cache de feed (`O(1)`)
    pub async fn obter_epoch_feed(&self) -> Option<u64> {
        let val = self.get_string(FEED_EPOCH_KEY).await;
        if let Some(s) = val {
            return Some(s.trim().parse::<u64>().unwrap_or(0));
        }
        // Verifica se o Redis responde antes de assumir epoch 0
        if self.ping().await {
            Some(0)
        } else {
            None
        }
    }

    /// Invalida todas as páginas cacheadas de `/api/v1/feed` em `O(1)` incrementando a geração
    pub async fn invalidar_cache_feed(&self) {
        let _ = self
            .executar_comando(&[b"INCR", FEED_EPOCH_KEY.as_bytes()])
            .await;
    }

    async fn executar_comando(&self, args: &[&[u8]]) -> Option<RespValue> {
        let endpoint = self.endpoint.as_ref()?;
        let payload = serializar_comando_resp(args);

        let mut guard = self.conn.lock().await;

        if let Some(reader) = guard.as_mut() {
            if let Ok(Ok(resp)) =
                timeout(REDIS_IO_TIMEOUT, enviar_e_ler(reader, &payload)).await
            {
                match resp {
                    RespValue::Error(_) => {}
                    valid => return Some(valid),
                }
            }
            *guard = None;
        }

        let mut novo_reader = conectar_endpoint(endpoint).await?;
        match timeout(REDIS_IO_TIMEOUT, enviar_e_ler(&mut novo_reader, &payload)).await {
            Ok(Ok(resp)) if !matches!(resp, RespValue::Error(_)) => {
                *guard = Some(novo_reader);
                Some(resp)
            }
            _ => None,
        }
    }
}

async fn conectar_endpoint(ep: &RedisEndpoint) -> Option<BufReader<TcpStream>> {
    let stream = timeout(REDIS_IO_TIMEOUT, TcpStream::connect((ep.host.as_str(), ep.port)))
        .await
        .ok()?
        .ok()?;
    let _ = stream.set_nodelay(true);
    let mut reader = BufReader::new(stream);

    if let Some(ref password) = ep.password {
        let auth_cmd = if let Some(ref user) = ep.username {
            serializar_comando_resp(&[b"AUTH", user.as_bytes(), password.as_bytes()])
        } else {
            serializar_comando_resp(&[b"AUTH", password.as_bytes()])
        };
        match timeout(REDIS_IO_TIMEOUT, enviar_e_ler(&mut reader, &auth_cmd)).await {
            Ok(Ok(RespValue::SimpleString(_))) => {}
            _ => return None,
        }
    }

    if let Some(db) = ep.db {
        let db_str = db.to_string();
        let sel_cmd = serializar_comando_resp(&[b"SELECT", db_str.as_bytes()]);
        match timeout(REDIS_IO_TIMEOUT, enviar_e_ler(&mut reader, &sel_cmd)).await {
            Ok(Ok(RespValue::SimpleString(_))) => {}
            _ => return None,
        }
    }

    Some(reader)
}

async fn enviar_e_ler(
    reader: &mut BufReader<TcpStream>,
    payload: &[u8],
) -> io::Result<RespValue> {
    reader.get_mut().write_all(payload).await?;
    reader.get_mut().flush().await?;
    ler_frame_resp(reader).await
}

fn serializar_comando_resp(args: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::with_capacity(32 * args.len());
    out.extend_from_slice(format!("*{}\r\n", args.len()).as_bytes());
    for arg in args {
        out.extend_from_slice(format!("${}\r\n", arg.len()).as_bytes());
        out.extend_from_slice(arg);
        out.extend_from_slice(b"\r\n");
    }
    out
}

async fn ler_frame_resp(reader: &mut BufReader<TcpStream>) -> io::Result<RespValue> {
    let mut line = String::new();
    let bytes_read = reader.read_line(&mut line).await?;
    if bytes_read == 0 {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "Conexão Redis encerrada",
        ));
    }

    let trimmed = line.trim_end_matches(['\r', '\n']);
    let Some(prefix) = trimmed.as_bytes().first().copied() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Frame RESP vazio",
        ));
    };
    let rest = &trimmed[1..];

    match prefix {
        b'+' => Ok(RespValue::SimpleString(rest.to_string())),
        b'-' => Ok(RespValue::Error(rest.to_string())),
        b':' => {
            let val = rest.parse::<i64>().map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "Inteiro RESP inválido")
            })?;
            Ok(RespValue::Integer(val))
        }
        b'$' => {
            let len = rest.parse::<isize>().map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "Tamanho Bulk RESP inválido")
            })?;
            if len < 0 {
                return Ok(RespValue::BulkBytes(None));
            }
            let size = len as usize;
            if size > MAX_BULK_BYTES {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Payload RESP excede limite máximo permitido",
                ));
            }
            let mut data = vec![0u8; size];
            reader.read_exact(&mut data).await?;
            let mut crlf = [0u8; 2];
            reader.read_exact(&mut crlf).await?;
            Ok(RespValue::BulkBytes(Some(data)))
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Prefixo RESP não suportado",
        )),
    }
}

pub fn parse_redis_url(raw: &str) -> Option<RedisEndpoint> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let without_scheme = trimmed
        .strip_prefix("redis://")
        .unwrap_or(trimmed);
    if without_scheme.is_empty() {
        return None;
    }

    let (authority, path_part) = match without_scheme.split_once('/') {
        Some((a, p)) => (a, p),
        None => (without_scheme, ""),
    };

    let db = path_part
        .trim()
        .parse::<u32>()
        .ok()
        .filter(|&idx| idx > 0);

    let (username, password, hostport) = match authority.rsplit_once('@') {
        Some((userinfo, hp)) => match userinfo.split_once(':') {
            Some((u, p)) => (
                (!u.is_empty()).then(|| u.to_string()),
                (!p.is_empty()).then(|| p.to_string()),
                hp,
            ),
            None => (
                None,
                (!userinfo.is_empty()).then(|| userinfo.to_string()),
                hp,
            ),
        },
        None => (None, None, authority),
    };

    let (host, port) = if let Some(after_bracket) = hostport.strip_prefix('[') {
        let (ipv6_host, rem) = after_bracket.split_once(']')?;
        let port = match rem.strip_prefix(':') {
            Some(p_str) => p_str.parse::<u16>().ok()?,
            None => 6379,
        };
        (ipv6_host.to_string(), port)
    } else if let Some((h, p_str)) = hostport.rsplit_once(':') {
        (h.trim().to_string(), p_str.trim().parse::<u16>().ok()?)
    } else {
        (hostport.trim().to_string(), 6379)
    };

    if host.is_empty() {
        return None;
    }

    Some(RedisEndpoint {
        host,
        port,
        username,
        password,
        db,
    })
}

#[cfg(test)]
mod tests {
    use super::{parse_redis_url, serializar_comando_resp, RedisService};
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
    use tokio::net::TcpListener;

    #[test]
    fn parser_redis_url_suporta_formatos_padrao_e_autenticados() {
        let simples = parse_redis_url("redis://localhost:6379").expect("url simples");
        assert_eq!(simples.host, "localhost");
        assert_eq!(simples.port, 6379);
        assert_eq!(simples.password, None);
        assert_eq!(simples.db, None);

        let autenticado =
            parse_redis_url("redis://default:segredo123@redis.interno:6380/2").expect("url auth");
        assert_eq!(autenticado.host, "redis.interno");
        assert_eq!(autenticado.port, 6380);
        assert_eq!(autenticado.username.as_deref(), Some("default"));
        assert_eq!(autenticado.password.as_deref(), Some("segredo123"));
        assert_eq!(autenticado.db, Some(2));

        assert!(parse_redis_url("").is_none());
    }

    #[test]
    fn serializador_resp_formata_array_bulk_strings() {
        let bytes = serializar_comando_resp(&[b"SETEX", b"chave", b"20", b"valor"]);
        assert_eq!(
            bytes,
            b"*4\r\n$5\r\nSETEX\r\n$5\r\nchave\r\n$2\r\n20\r\n$5\r\nvalor\r\n"
        );
    }

    #[tokio::test]
    async fn cliente_resp2_opera_contra_servidor_mock_local() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind mock");
        let addr = listener.local_addr().expect("addr mock");

        tokio::spawn(async move {
            let (socket, _) = listener.accept().await.expect("accept mock");
            let mut reader = BufReader::new(socket);
            let mut store = std::collections::HashMap::<String, String>::new();

            loop {
                let mut header = String::new();
                if reader.read_line(&mut header).await.unwrap_or(0) == 0 {
                    break;
                }
                let argc: usize = header
                    .trim()
                    .strip_prefix('*')
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                let mut args = Vec::with_capacity(argc);
                for _ in 0..argc {
                    let mut len_line = String::new();
                    reader.read_line(&mut len_line).await.expect("len");
                    let n: usize = len_line
                        .trim()
                        .strip_prefix('$')
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);
                    let mut buf = vec![0u8; n];
                    reader.read_exact(&mut buf).await.expect("arg");
                    let mut crlf = [0u8; 2];
                    reader.read_exact(&mut crlf).await.expect("crlf");
                    args.push(String::from_utf8_lossy(&buf).to_string());
                }

                let cmd = args.first().map(|s| s.to_uppercase()).unwrap_or_default();
                let stream = reader.get_mut();
                match cmd.as_str() {
                    "PING" => {
                        stream.write_all(b"+PONG\r\n").await.expect("pong");
                    }
                    "SETEX" => {
                        store.insert(args[1].clone(), args[3].clone());
                        stream.write_all(b"+OK\r\n").await.expect("ok");
                    }
                    "GET" => match store.get(&args[1]) {
                        Some(v) => {
                            let msg = format!("${}\r\n{}\r\n", v.len(), v);
                            stream.write_all(msg.as_bytes()).await.expect("val");
                        }
                        None => {
                            stream.write_all(b"$-1\r\n").await.expect("nil");
                        }
                    },
                    "INCR" => {
                        let next = store
                            .get(&args[1])
                            .and_then(|v| v.parse::<i64>().ok())
                            .unwrap_or(0)
                            + 1;
                        store.insert(args[1].clone(), next.to_string());
                        stream
                            .write_all(format!(":{}\r\n", next).as_bytes())
                            .await
                            .expect("int");
                    }
                    "EXPIRE" => {
                        stream.write_all(b":1\r\n").await.expect("exp");
                    }
                    _ => {
                        stream.write_all(b"-ERR unknown\r\n").await.expect("err");
                    }
                }
                stream.flush().await.expect("flush");
            }
        });

        let svc = RedisService::from_url(&format!("redis://{}", addr));
        assert!(svc.ping().await);
        assert!(svc.set_ex("teste:k", 20, "{\"ok\":true}").await);
        assert_eq!(svc.get_string("teste:k").await.as_deref(), Some("{\"ok\":true}"));
        assert_eq!(svc.incr_window("teste:rl", 60).await, Some(1));
        assert_eq!(svc.incr_window("teste:rl", 60).await, Some(2));
        assert_eq!(svc.obter_epoch_feed().await, Some(0));
        svc.invalidar_cache_feed().await;
        assert_eq!(svc.obter_epoch_feed().await, Some(1));
    }
}
