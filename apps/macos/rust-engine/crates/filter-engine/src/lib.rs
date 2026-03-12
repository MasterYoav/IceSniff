use session_model::{
    ApplicationLayerSummary, DecodedPacket, NetworkLayerSummary, StreamRow, TransactionDetail,
    TransactionRow, TransportLayerSummary,
};

pub fn matches_filter(packet: &DecodedPacket, expression: &str) -> Result<bool, String> {
    let expr = parse_expression(expression)?;
    eval_expr(packet, &expr, packet_field_values)
}

pub fn matches_stream_filter(row: &StreamRow, expression: &str) -> Result<bool, String> {
    let expr = parse_expression(expression)?;
    eval_expr(row, &expr, stream_field_values)
}

pub fn matches_transaction_filter(row: &TransactionRow, expression: &str) -> Result<bool, String> {
    let expr = parse_expression(expression)?;
    eval_expr(row, &expr, transaction_field_values)
}

fn parse_expression(expression: &str) -> Result<Expr, String> {
    let normalized = expression.replace(',', " && ");
    let tokens = tokenize(&normalized)?;
    if tokens.is_empty() {
        return Err("filter expression is empty".to_string());
    }

    let mut parser = Parser {
        tokens: &tokens,
        index: 0,
    };
    let expr = parser.parse_expression()?;
    if parser.peek().is_some() {
        return Err(format!(
            "unexpected trailing token: {}",
            parser.peek().unwrap_or(&Token::End).display()
        ));
    }

    Ok(expr)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Expr {
    Clause {
        key: String,
        operator: ClauseOperator,
        value: String,
    },
    Not(Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ClauseOperator {
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
    Contains,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Clause(String),
    And,
    Or,
    Not,
    LParen,
    RParen,
    End,
}

impl Token {
    fn display(&self) -> &str {
        match self {
            Token::Clause(value) => value,
            Token::And => "&&",
            Token::Or => "||",
            Token::Not => "!",
            Token::LParen => "(",
            Token::RParen => ")",
            Token::End => "<end>",
        }
    }
}

struct Parser<'a> {
    tokens: &'a [Token],
    index: usize,
}

impl<'a> Parser<'a> {
    fn parse_expression(&mut self) -> Result<Expr, String> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_and()?;
        while matches!(self.peek(), Some(Token::Or)) {
            self.index += 1;
            let right = self.parse_and()?;
            left = Expr::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_not()?;
        while matches!(self.peek(), Some(Token::And)) {
            self.index += 1;
            let right = self.parse_not()?;
            left = Expr::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_not(&mut self) -> Result<Expr, String> {
        if matches!(self.peek(), Some(Token::Not)) {
            self.index += 1;
            return Ok(Expr::Not(Box::new(self.parse_not()?)));
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.next() {
            Some(Token::Clause(clause)) => {
                let (key, operator, value) = parse_clause(&clause)?;
                Ok(Expr::Clause {
                    key,
                    operator,
                    value,
                })
            }
            Some(Token::LParen) => {
                let expr = self.parse_expression()?;
                match self.next() {
                    Some(Token::RParen) => Ok(expr),
                    Some(token) => Err(format!("expected ')', found {}", token.display())),
                    None => Err("expected ')' at end of filter expression".to_string()),
                }
            }
            Some(token) => Err(format!("unexpected token: {}", token.display())),
            None => Err("unexpected end of filter expression".to_string()),
        }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.index)
    }

    fn next(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.index)?.clone();
        self.index += 1;
        Some(token)
    }
}

fn tokenize(expression: &str) -> Result<Vec<Token>, String> {
    let chars = expression.chars().collect::<Vec<_>>();
    let mut tokens = Vec::new();
    let mut index = 0usize;

    while index < chars.len() {
        match chars[index] {
            c if c.is_whitespace() => index += 1,
            '(' => {
                tokens.push(Token::LParen);
                index += 1;
            }
            ')' => {
                tokens.push(Token::RParen);
                index += 1;
            }
            '!' if chars.get(index + 1) != Some(&'=') => {
                tokens.push(Token::Not);
                index += 1;
            }
            '&' => {
                if chars.get(index + 1) == Some(&'&') {
                    tokens.push(Token::And);
                    index += 2;
                } else {
                    return Err("single '&' is not supported; use '&&'".to_string());
                }
            }
            '|' => {
                if chars.get(index + 1) == Some(&'|') {
                    tokens.push(Token::Or);
                    index += 2;
                } else {
                    return Err("single '|' is not supported; use '||'".to_string());
                }
            }
            _ => {
                let start = index;
                while index < chars.len()
                    && !chars[index].is_whitespace()
                    && !matches!(chars[index], '(' | ')' | '&' | '|')
                    && !(chars[index] == '!' && chars.get(index + 1) != Some(&'='))
                {
                    index += 1;
                }
                let token = chars[start..index].iter().collect::<String>();
                match token.as_str() {
                    "and" => tokens.push(Token::And),
                    "or" => tokens.push(Token::Or),
                    "not" => tokens.push(Token::Not),
                    _ => tokens.push(Token::Clause(token)),
                }
            }
        }
    }

    Ok(tokens)
}

fn parse_clause(clause: &str) -> Result<(String, ClauseOperator, String), String> {
    for (operator_text, operator) in [
        ("!=", ClauseOperator::Ne),
        (">=", ClauseOperator::Ge),
        ("<=", ClauseOperator::Le),
        ("~=", ClauseOperator::Contains),
        ("=", ClauseOperator::Eq),
        (">", ClauseOperator::Gt),
        ("<", ClauseOperator::Lt),
    ] {
        if let Some((key, value)) = clause.split_once(operator_text) {
            let key = key.trim().to_string();
            let value = value.trim().to_string();
            if key.is_empty() || value.is_empty() {
                return Err(format!("invalid filter clause: {clause}"));
            }
            return Ok((key, operator, value));
        }
    }
    Err(format!("invalid filter clause: {clause}"))
}

fn eval_expr<T>(
    target: &T,
    expr: &Expr,
    field_values_fn: fn(&T, &str) -> Result<Vec<FieldValue>, String>,
) -> Result<bool, String> {
    match expr {
        Expr::Clause {
            key,
            operator,
            value: query,
        } => matches_clause(target, key, operator, query, field_values_fn),
        Expr::Not(inner) => Ok(!eval_expr(target, inner, field_values_fn)?),
        Expr::And(left, right) => {
            Ok(eval_expr(target, left, field_values_fn)?
                && eval_expr(target, right, field_values_fn)?)
        }
        Expr::Or(left, right) => {
            Ok(eval_expr(target, left, field_values_fn)?
                || eval_expr(target, right, field_values_fn)?)
        }
    }
}

fn matches_clause<T>(
    value_source: &T,
    key: &str,
    operator: &ClauseOperator,
    value: &str,
    field_values_fn: fn(&T, &str) -> Result<Vec<FieldValue>, String>,
) -> Result<bool, String> {
    let values = field_values_fn(value_source, key)?;
    Ok(values
        .iter()
        .any(|field_value| compare_field_value(field_value, operator, value)))
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum FieldValue {
    Text(String),
    Number(i64),
}

fn packet_field_values(packet: &DecodedPacket, key: &str) -> Result<Vec<FieldValue>, String> {
    match key {
        "protocol" | "service" => Ok(packet_protocol_values(packet)
            .into_iter()
            .map(FieldValue::Text)
            .collect()),
        "port" => Ok(packet_ports(packet)
            .into_iter()
            .map(|port| FieldValue::Number(i64::from(port)))
            .collect()),
        "host" => Ok(packet_hosts(packet)
            .into_iter()
            .map(FieldValue::Text)
            .collect()),
        "ip" => Ok(packet_ips(packet)
            .into_iter()
            .map(FieldValue::Text)
            .collect()),
        "endpoint" => Ok(packet_endpoints(packet)
            .into_iter()
            .map(FieldValue::Text)
            .collect()),
        "http.method" => Ok(packet_http_method(packet)
            .into_iter()
            .map(FieldValue::Text)
            .collect()),
        "http.path" => Ok(packet_http_path(packet)
            .into_iter()
            .map(FieldValue::Text)
            .collect()),
        "http.kind" => Ok(packet_http_kind(packet)
            .into_iter()
            .map(FieldValue::Text)
            .collect()),
        "http.status" => Ok(packet_http_status(packet)
            .into_iter()
            .map(FieldValue::Number)
            .collect()),
        "http.reason" => Ok(packet_http_reason(packet)
            .into_iter()
            .map(FieldValue::Text)
            .collect()),
        "http.host" => Ok(packet_http_host(packet)
            .into_iter()
            .map(FieldValue::Text)
            .collect()),
        "dns.question" => Ok(packet_dns_questions(packet)
            .into_iter()
            .map(FieldValue::Text)
            .collect()),
        "dns.question_count" => Ok(packet_dns_question_count(packet)
            .into_iter()
            .map(FieldValue::Number)
            .collect()),
        "dns.answer_count" => Ok(packet_dns_answer_count(packet)
            .into_iter()
            .map(FieldValue::Number)
            .collect()),
        "dns.is_response" => Ok(packet_dns_is_response(packet)
            .into_iter()
            .map(FieldValue::Text)
            .collect()),
        "tls.handshake_type" => Ok(packet_tls_handshake_type(packet)
            .into_iter()
            .map(FieldValue::Text)
            .collect()),
        "tls.server_name" => Ok(packet_tls_server_name(packet)
            .into_iter()
            .map(FieldValue::Text)
            .collect()),
        "tls.record_version" => Ok(packet_tls_record_version(packet)
            .into_iter()
            .map(FieldValue::Text)
            .collect()),
        "tls.handshake_length" => Ok(packet_tls_handshake_length(packet)
            .into_iter()
            .map(FieldValue::Number)
            .collect()),
        _ => Err(format!("unsupported filter key: {key}")),
    }
}

fn stream_field_values(row: &StreamRow, key: &str) -> Result<Vec<FieldValue>, String> {
    match key {
        "stream.service" => Ok(text_values([row.service.clone()])),
        "stream.protocol" => Ok(text_values([row.protocol.clone()])),
        "stream.client" => Ok(text_values([row.client.clone()])),
        "stream.server" => Ok(text_values([row.server.clone()])),
        "stream.state" => Ok(text_values([row.session_state.clone()])),
        "stream.tls_state" => Ok(text_values([row.tls_handshake_state.clone()])),
        "stream.packets" => Ok(number_values([row.packets])),
        "stream.client_packets" => Ok(number_values([row.client_to_server_packets])),
        "stream.server_packets" => Ok(number_values([row.server_to_client_packets])),
        "stream.syn" => Ok(number_values([row.syn_packets])),
        "stream.fin" => Ok(number_values([row.fin_packets])),
        "stream.rst" => Ok(number_values([row.rst_packets])),
        "stream.requests" => Ok(number_values([row.request_count])),
        "stream.responses" => Ok(number_values([row.response_count])),
        "stream.matched" => Ok(number_values([row.matched_transactions])),
        "stream.unmatched_requests" => Ok(number_values([row.unmatched_requests])),
        "stream.unmatched_responses" => Ok(number_values([row.unmatched_responses])),
        "stream.tls_client_hellos" => Ok(number_values([row.tls_client_hellos])),
        "stream.tls_server_hellos" => Ok(number_values([row.tls_server_hellos])),
        "stream.tls_certificates" => Ok(number_values([row.tls_certificates])),
        "stream.tls_finished" => Ok(number_values([row.tls_finished_messages])),
        "stream.tls_handshake_cycles" => Ok(number_values([row.tls_handshake_cycles])),
        "stream.tls_incomplete_handshakes" => Ok(number_values([row.tls_incomplete_handshakes])),
        "stream.tls_alert_count" => Ok(number_values([row.tls_alert_count])),
        "stream.tls_alert" => Ok(text_values(row.tls_alerts.clone())),
        "stream.has_alerts" => Ok(text_values([bool_text(row.tls_alert_count > 0)])),
        "stream.has_timeline" => Ok(text_values([bool_text(
            !row.transaction_timeline.is_empty(),
        )])),
        "stream.has_notes" => Ok(text_values([bool_text(
            !row.notes.is_empty() || !row.tls_alerts.is_empty(),
        )])),
        "stream.has_reassembly_issues" => {
            Ok(text_values([bool_text(stream_has_reassembly_issues(row))]))
        }
        "stream.is_pipelined" => Ok(text_values([bool_text(stream_is_pipelined(row))])),
        "stream.total_bytes" => Ok(number_values([row.total_captured_bytes])),
        "stream.first_packet" => Ok(number_values([row.first_packet_index])),
        "stream.last_packet" => Ok(number_values([row.last_packet_index])),
        "stream.timeline" => Ok(text_values(row.transaction_timeline.clone())),
        "stream.note" => {
            let mut values = row.notes.clone();
            values.extend(row.tls_alerts.clone());
            Ok(text_values(values))
        }
        _ => Err(format!("unsupported filter key: {key}")),
    }
}

fn transaction_field_values(row: &TransactionRow, key: &str) -> Result<Vec<FieldValue>, String> {
    match key {
        "tx.service" => Ok(text_values([row.service.clone()])),
        "tx.protocol" => Ok(text_values([row.protocol.clone()])),
        "tx.client" => Ok(text_values([row.client.clone()])),
        "tx.server" => Ok(text_values([row.server.clone()])),
        "tx.sequence" => Ok(number_values([row.sequence])),
        "tx.state" => Ok(text_values([row.state.clone()])),
        "tx.has_request" => Ok(text_values([bool_text(row.request_summary != "none")])),
        "tx.has_response" => Ok(text_values([bool_text(row.response_summary != "none")])),
        "tx.complete" => Ok(text_values([bool_text(
            row.request_summary != "none" && row.response_summary != "none",
        )])),
        "tx.has_alerts" => Ok(text_values([bool_text(transaction_has_alerts(row))])),
        "tx.http.status_class" => transaction_http_status_class_values(row),
        "tx.request_summary" => Ok(text_values([row.request_summary.clone()])),
        "tx.response_summary" => Ok(text_values([row.response_summary.clone()])),
        "tx.note" => Ok(text_values(row.notes.clone())),
        "tx.http.method" => detail_field_values(&row.request_details, "method"),
        "tx.http.path" => detail_field_values(&row.request_details, "path"),
        "tx.http.host" => detail_field_values(&row.request_details, "host"),
        "tx.http.status" => detail_field_values(&row.response_details, "status_code"),
        "tx.http.reason" => detail_field_values(&row.response_details, "reason_phrase"),
        "tx.http.transfer_semantics" => combined_detail_field_values(
            [&row.request_details, &row.response_details],
            "transfer_semantics",
        ),
        "tx.http.transfer_encoding" => combined_detail_field_values(
            [&row.request_details, &row.response_details],
            "transfer_encoding",
        ),
        "tx.http.content_type" => combined_detail_field_values(
            [&row.request_details, &row.response_details],
            "content_type",
        ),
        "tx.http.body_bytes" => combined_detail_field_values(
            [&row.request_details, &row.response_details],
            "body_bytes",
        ),
        "tx.http.header_count" => combined_detail_field_values(
            [&row.request_details, &row.response_details],
            "header_count",
        ),
        "tx.tls.record_version" => combined_detail_field_values(
            [&row.request_details, &row.response_details],
            "record_version",
        ),
        "tx.tls.server_name" => detail_field_values(&row.request_details, "server_name"),
        "tx.tls.alpn" => detail_field_values(&row.request_details, "alpn"),
        "tx.tls.handshake_messages" => combined_detail_field_values(
            [&row.request_details, &row.response_details],
            "handshake_messages",
        ),
        "tx.tls.alerts" => {
            combined_detail_field_values([&row.request_details, &row.response_details], "alerts")
        }
        "tx.tls.certificate_messages" => {
            detail_field_values(&row.response_details, "certificate_messages")
        }
        _ if key.starts_with("tx.request.") => {
            detail_field_values(&row.request_details, &key["tx.request.".len()..])
        }
        _ if key.starts_with("tx.response.") => {
            detail_field_values(&row.response_details, &key["tx.response.".len()..])
        }
        _ => Err(format!("unsupported filter key: {key}")),
    }
}

fn detail_field_values(
    details: &[TransactionDetail],
    key: &str,
) -> Result<Vec<FieldValue>, String> {
    if !supported_transaction_detail_key(key) {
        return Err(format!("unsupported filter key: tx detail field {key}"));
    }

    let values = details
        .iter()
        .filter(|detail| detail.key == key)
        .map(|detail| {
            detail
                .value
                .parse::<i64>()
                .map(FieldValue::Number)
                .unwrap_or_else(|_| FieldValue::Text(detail.value.clone()))
        })
        .collect::<Vec<_>>();
    Ok(values)
}

fn combined_detail_field_values<const N: usize>(
    detail_sets: [&[TransactionDetail]; N],
    key: &str,
) -> Result<Vec<FieldValue>, String> {
    if !supported_transaction_detail_key(key) {
        return Err(format!("unsupported filter key: tx detail field {key}"));
    }

    let values = detail_sets
        .into_iter()
        .flat_map(|details| detail_field_values_from_supported_key(details, key))
        .collect::<Vec<_>>();
    Ok(values)
}

fn supported_transaction_detail_key(key: &str) -> bool {
    matches!(
        key,
        "method"
            | "path"
            | "host"
            | "status_code"
            | "reason_phrase"
            | "header_count"
            | "body_bytes"
            | "transfer_semantics"
            | "transfer_encoding"
            | "content_type"
            | "record_version"
            | "server_name"
            | "alpn"
            | "handshake_messages"
            | "alerts"
            | "certificate_messages"
    )
}

fn detail_field_values_from_supported_key(
    details: &[TransactionDetail],
    key: &str,
) -> Vec<FieldValue> {
    details
        .iter()
        .filter(|detail| detail.key == key)
        .map(|detail| {
            detail
                .value
                .parse::<i64>()
                .map(FieldValue::Number)
                .unwrap_or_else(|_| FieldValue::Text(detail.value.clone()))
        })
        .collect()
}

fn stream_has_reassembly_issues(row: &StreamRow) -> bool {
    row.notes.iter().any(|note| {
        let note = note.to_ascii_lowercase();
        note.contains("sequence gap")
            || note.contains("retransmitted")
            || note.contains("overlapping")
            || note.contains("out-of-order")
            || note.contains("incomplete")
    })
}

fn stream_is_pipelined(row: &StreamRow) -> bool {
    row.notes
        .iter()
        .any(|note| note.to_ascii_lowercase().contains("pipelined requests"))
}

fn transaction_has_alerts(row: &TransactionRow) -> bool {
    row.request_details
        .iter()
        .chain(row.response_details.iter())
        .any(|detail| detail.key == "alerts" && !detail.value.is_empty())
}

fn transaction_http_status_class_values(row: &TransactionRow) -> Result<Vec<FieldValue>, String> {
    let values = detail_field_values(&row.response_details, "status_code")?
        .into_iter()
        .filter_map(|value| match value {
            FieldValue::Number(status_code) => Some(FieldValue::Number(status_code / 100)),
            FieldValue::Text(_) => None,
        })
        .collect::<Vec<_>>();
    Ok(values)
}

fn text_values<I>(values: I) -> Vec<FieldValue>
where
    I: IntoIterator<Item = String>,
{
    values.into_iter().map(FieldValue::Text).collect()
}

fn number_values<I>(values: I) -> Vec<FieldValue>
where
    I: IntoIterator<Item = u64>,
{
    values
        .into_iter()
        .filter_map(|value| i64::try_from(value).ok())
        .map(FieldValue::Number)
        .collect()
}

fn bool_text(value: bool) -> String {
    if value {
        "true".to_string()
    } else {
        "false".to_string()
    }
}

fn compare_field_value(field_value: &FieldValue, operator: &ClauseOperator, query: &str) -> bool {
    match (field_value, operator) {
        (FieldValue::Text(value), ClauseOperator::Eq) => eq_ignore_ascii_case(value, query),
        (FieldValue::Text(value), ClauseOperator::Ne) => !eq_ignore_ascii_case(value, query),
        (FieldValue::Text(value), ClauseOperator::Contains) => value
            .to_ascii_lowercase()
            .contains(&query.to_ascii_lowercase()),
        (FieldValue::Number(value), ClauseOperator::Eq) => query.parse::<i64>() == Ok(*value),
        (FieldValue::Number(value), ClauseOperator::Ne) => {
            query.parse::<i64>().is_ok_and(|q| q != *value)
        }
        (FieldValue::Number(value), ClauseOperator::Gt) => {
            query.parse::<i64>().is_ok_and(|q| *value > q)
        }
        (FieldValue::Number(value), ClauseOperator::Ge) => {
            query.parse::<i64>().is_ok_and(|q| *value >= q)
        }
        (FieldValue::Number(value), ClauseOperator::Lt) => {
            query.parse::<i64>().is_ok_and(|q| *value < q)
        }
        (FieldValue::Number(value), ClauseOperator::Le) => {
            query.parse::<i64>().is_ok_and(|q| *value <= q)
        }
        (FieldValue::Text(_), ClauseOperator::Gt)
        | (FieldValue::Text(_), ClauseOperator::Ge)
        | (FieldValue::Text(_), ClauseOperator::Lt)
        | (FieldValue::Text(_), ClauseOperator::Le)
        | (FieldValue::Number(_), ClauseOperator::Contains) => false,
    }
}

fn eq_ignore_ascii_case(left: &str, right: &str) -> bool {
    left.eq_ignore_ascii_case(right)
}

fn packet_protocol_values(packet: &DecodedPacket) -> Vec<String> {
    let mut values = Vec::new();

    match &packet.application {
        Some(ApplicationLayerSummary::Dns(_)) => values.push("dns".to_string()),
        Some(ApplicationLayerSummary::Http(_)) => values.push("http".to_string()),
        Some(ApplicationLayerSummary::TlsHandshake(_)) => values.push("tls".to_string()),
        None => {}
    }

    for port in packet_ports(packet) {
        match port {
            53 | 5353 => values.push("dns".to_string()),
            80 => values.push("http".to_string()),
            443 => values.push("tls".to_string()),
            _ => {}
        }
    }

    match &packet.transport {
        Some(TransportLayerSummary::Tcp(_)) => values.push("tcp".to_string()),
        Some(TransportLayerSummary::Udp(_)) => values.push("udp".to_string()),
        Some(TransportLayerSummary::Icmp(_)) => values.push("icmp".to_string()),
        None => match &packet.network {
            Some(NetworkLayerSummary::Arp(_)) => values.push("arp".to_string()),
            Some(NetworkLayerSummary::Ipv4(_)) => values.push("ipv4".to_string()),
            Some(NetworkLayerSummary::Ipv6(_)) => values.push("ipv6".to_string()),
            None => values.push("unknown".to_string()),
        },
    }

    values.sort();
    values.dedup();
    values
}

fn packet_ports(packet: &DecodedPacket) -> Vec<u16> {
    match &packet.transport {
        Some(TransportLayerSummary::Tcp(tcp)) => vec![tcp.source_port, tcp.destination_port],
        Some(TransportLayerSummary::Udp(udp)) => vec![udp.source_port, udp.destination_port],
        _ => Vec::new(),
    }
}

fn packet_hosts(packet: &DecodedPacket) -> Vec<String> {
    let mut hosts = packet_ips(packet);
    match &packet.application {
        Some(ApplicationLayerSummary::Dns(dns)) => hosts.extend(dns.questions.clone()),
        Some(ApplicationLayerSummary::Http(http)) => {
            if let Some(host) = &http.host {
                hosts.push(host.clone());
            }
        }
        Some(ApplicationLayerSummary::TlsHandshake(tls)) => {
            if let Some(server_name) = &tls.server_name {
                hosts.push(server_name.clone());
            }
        }
        _ => {}
    }
    hosts
}

fn packet_ips(packet: &DecodedPacket) -> Vec<String> {
    match &packet.network {
        Some(NetworkLayerSummary::Ipv4(ipv4)) => {
            vec![ipv4.source_ip.clone(), ipv4.destination_ip.clone()]
        }
        Some(NetworkLayerSummary::Ipv6(ipv6)) => {
            vec![ipv6.source_ip.clone(), ipv6.destination_ip.clone()]
        }
        Some(NetworkLayerSummary::Arp(arp)) => vec![
            arp.sender_protocol_address.clone(),
            arp.target_protocol_address.clone(),
        ],
        _ => Vec::new(),
    }
}

fn packet_endpoints(packet: &DecodedPacket) -> Vec<String> {
    match (&packet.network, &packet.transport) {
        (Some(NetworkLayerSummary::Ipv4(ipv4)), Some(TransportLayerSummary::Tcp(tcp))) => {
            vec![
                format!("{}:{}", ipv4.source_ip, tcp.source_port),
                format!("{}:{}", ipv4.destination_ip, tcp.destination_port),
            ]
        }
        (Some(NetworkLayerSummary::Ipv6(ipv6)), Some(TransportLayerSummary::Tcp(tcp))) => {
            vec![
                format!("{}:{}", ipv6.source_ip, tcp.source_port),
                format!("{}:{}", ipv6.destination_ip, tcp.destination_port),
            ]
        }
        (Some(NetworkLayerSummary::Ipv4(ipv4)), Some(TransportLayerSummary::Udp(udp))) => {
            vec![
                format!("{}:{}", ipv4.source_ip, udp.source_port),
                format!("{}:{}", ipv4.destination_ip, udp.destination_port),
            ]
        }
        (Some(NetworkLayerSummary::Ipv6(ipv6)), Some(TransportLayerSummary::Udp(udp))) => {
            vec![
                format!("{}:{}", ipv6.source_ip, udp.source_port),
                format!("{}:{}", ipv6.destination_ip, udp.destination_port),
            ]
        }
        _ => Vec::new(),
    }
}

fn packet_http_method(packet: &DecodedPacket) -> Vec<String> {
    match &packet.application {
        Some(ApplicationLayerSummary::Http(http)) => http.method.iter().cloned().collect(),
        _ => Vec::new(),
    }
}

fn packet_http_path(packet: &DecodedPacket) -> Vec<String> {
    match &packet.application {
        Some(ApplicationLayerSummary::Http(http)) => http.path.iter().cloned().collect(),
        _ => Vec::new(),
    }
}

fn packet_http_kind(packet: &DecodedPacket) -> Vec<String> {
    match &packet.application {
        Some(ApplicationLayerSummary::Http(http)) => vec![http.kind.clone()],
        _ => Vec::new(),
    }
}

fn packet_http_status(packet: &DecodedPacket) -> Vec<i64> {
    match &packet.application {
        Some(ApplicationLayerSummary::Http(http)) => {
            http.status_code.into_iter().map(i64::from).collect()
        }
        _ => Vec::new(),
    }
}

fn packet_http_reason(packet: &DecodedPacket) -> Vec<String> {
    match &packet.application {
        Some(ApplicationLayerSummary::Http(http)) => http.reason_phrase.iter().cloned().collect(),
        _ => Vec::new(),
    }
}

fn packet_http_host(packet: &DecodedPacket) -> Vec<String> {
    match &packet.application {
        Some(ApplicationLayerSummary::Http(http)) => http.host.iter().cloned().collect(),
        _ => Vec::new(),
    }
}

fn packet_dns_questions(packet: &DecodedPacket) -> Vec<String> {
    match &packet.application {
        Some(ApplicationLayerSummary::Dns(dns)) => dns.questions.clone(),
        _ => Vec::new(),
    }
}

fn packet_dns_question_count(packet: &DecodedPacket) -> Vec<i64> {
    match &packet.application {
        Some(ApplicationLayerSummary::Dns(dns)) => vec![i64::from(dns.question_count)],
        _ => Vec::new(),
    }
}

fn packet_dns_answer_count(packet: &DecodedPacket) -> Vec<i64> {
    match &packet.application {
        Some(ApplicationLayerSummary::Dns(dns)) => vec![i64::from(dns.answer_count)],
        _ => Vec::new(),
    }
}

fn packet_dns_is_response(packet: &DecodedPacket) -> Vec<String> {
    match &packet.application {
        Some(ApplicationLayerSummary::Dns(dns)) => {
            vec![if dns.is_response { "true" } else { "false" }.to_string()]
        }
        _ => Vec::new(),
    }
}

fn packet_tls_handshake_type(packet: &DecodedPacket) -> Vec<String> {
    match &packet.application {
        Some(ApplicationLayerSummary::TlsHandshake(tls)) => vec![tls.handshake_type.clone()],
        _ => Vec::new(),
    }
}

fn packet_tls_server_name(packet: &DecodedPacket) -> Vec<String> {
    match &packet.application {
        Some(ApplicationLayerSummary::TlsHandshake(tls)) => {
            tls.server_name.iter().cloned().collect()
        }
        _ => Vec::new(),
    }
}

fn packet_tls_record_version(packet: &DecodedPacket) -> Vec<String> {
    match &packet.application {
        Some(ApplicationLayerSummary::TlsHandshake(tls)) => vec![tls.record_version.clone()],
        _ => Vec::new(),
    }
}

fn packet_tls_handshake_length(packet: &DecodedPacket) -> Vec<i64> {
    match &packet.application {
        Some(ApplicationLayerSummary::TlsHandshake(tls)) => vec![i64::from(tls.handshake_length)],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::{matches_filter, matches_stream_filter, matches_transaction_filter};
    use session_model::{
        ApplicationLayerSummary, DecodedPacket, DnsMessageSummary, EthernetFrameSummary, FieldNode,
        LinkLayerSummary, NetworkLayerSummary, PacketSummary, StreamRow, TimestampPrecision,
        TransactionDetail, TransactionRow, TransportLayerSummary, UdpDatagramSummary,
    };

    #[test]
    fn supports_or_expressions() {
        let packet = sample_dns_packet();
        assert!(matches_filter(&packet, "protocol=http || protocol=dns").unwrap());
    }

    #[test]
    fn supports_parenthesized_and_not_expressions() {
        let packet = sample_dns_packet();
        assert!(matches_filter(&packet, "protocol=dns && !(port=443 || host=other.test)").unwrap());
    }

    #[test]
    fn rejects_invalid_syntax() {
        let packet = sample_dns_packet();
        let error = matches_filter(&packet, "protocol=dns &&").unwrap_err();
        assert!(error.contains("unexpected end"));
    }

    #[test]
    fn supports_http_field_predicates() {
        let packet = sample_http_packet();
        assert!(matches_filter(
            &packet,
            "http.method=GET && http.path=/hello && http.host=example.com"
        )
        .unwrap());
    }

    #[test]
    fn supports_tls_field_predicates() {
        let packet = sample_tls_packet();
        assert!(matches_filter(
            &packet,
            "tls.handshake_type=client_hello && tls.server_name=example.com && tls.record_version=3.3"
        )
        .unwrap());
    }

    #[test]
    fn supports_numeric_comparisons() {
        let packet = sample_http_response_packet();
        assert!(matches_filter(&packet, "http.status>=200 && http.status<300").unwrap());
    }

    #[test]
    fn supports_contains_predicates() {
        let packet = sample_tls_packet();
        assert!(matches_filter(&packet, "tls.server_name~=example && host~=example").unwrap());
    }

    #[test]
    fn supports_case_insensitive_text_matches() {
        let packet = sample_http_packet();
        assert!(matches_filter(&packet, "http.method=get && host=EXAMPLE.COM").unwrap());
    }

    #[test]
    fn supports_additional_dns_and_http_fields() {
        let packet = sample_http_response_packet();
        assert!(matches_filter(&packet, "http.kind=response && http.reason~=content").unwrap());

        let dns_packet = sample_dns_packet();
        assert!(matches_filter(
            &dns_packet,
            "dns.question_count=1 && dns.answer_count=0 && dns.is_response=false"
        )
        .unwrap());
    }

    #[test]
    fn supports_tls_handshake_length_ranges() {
        let packet = sample_tls_packet();
        assert!(matches_filter(
            &packet,
            "tls.handshake_length>=60 && tls.handshake_length<70"
        )
        .unwrap());
    }

    #[test]
    fn supports_stream_row_filters() {
        let row = sample_stream_row();
        assert!(matches_stream_filter(
            &row,
            "stream.service=tls && stream.state=reset && stream.has_alerts=true && stream.tls_alert_count>=1"
        )
        .unwrap());
    }

    #[test]
    fn supports_transaction_row_filters() {
        let row = sample_transaction_row();
        assert!(matches_transaction_filter(
            &row,
            "tx.state=matched && tx.request.method=get && tx.response.status_code>=200 && tx.response.reason_phrase~=ok"
        )
        .unwrap());
    }

    #[test]
    fn supports_derived_stream_and_transaction_aliases() {
        let stream = sample_pipelined_stream_row();
        assert!(matches_stream_filter(
            &stream,
            "stream.is_pipelined=true && stream.has_reassembly_issues=true && stream.client_packets=2 && stream.total_bytes>=500"
        )
        .unwrap());

        let tls_transaction = sample_tls_transaction_row();
        assert!(matches_transaction_filter(
            &tls_transaction,
            "tx.has_alerts=true && tx.tls.alpn=h2 && tx.tls.alerts~=handshake_failure && tx.complete=true"
        )
        .unwrap());
    }

    fn sample_dns_packet() -> DecodedPacket {
        DecodedPacket {
            summary: PacketSummary {
                index: 0,
                timestamp_seconds: 0,
                timestamp_fraction: 0,
                timestamp_precision: TimestampPrecision::Microseconds,
                captured_length: 42,
                original_length: 42,
            },
            raw_bytes: Vec::new(),
            link: LinkLayerSummary::Ethernet(EthernetFrameSummary {
                source_mac: "00:11:22:33:44:55".to_string(),
                destination_mac: "66:77:88:99:aa:bb".to_string(),
                ether_type: 0x0800,
            }),
            network: Some(NetworkLayerSummary::Ipv4(
                session_model::Ipv4PacketSummary {
                    source_ip: "192.168.1.10".to_string(),
                    destination_ip: "8.8.8.8".to_string(),
                    protocol: 17,
                    ttl: 64,
                    header_length: 20,
                    total_length: 42,
                },
            )),
            transport: Some(TransportLayerSummary::Udp(UdpDatagramSummary {
                source_port: 5353,
                destination_port: 53,
                length: 8,
            })),
            application: Some(ApplicationLayerSummary::Dns(DnsMessageSummary {
                id: 1,
                is_response: false,
                opcode: 0,
                question_count: 1,
                answer_count: 0,
                questions: vec!["example.com".to_string()],
            })),
            fields: Vec::<FieldNode>::new(),
            notes: Vec::new(),
        }
    }

    fn sample_http_packet() -> DecodedPacket {
        DecodedPacket {
            summary: PacketSummary {
                index: 1,
                timestamp_seconds: 0,
                timestamp_fraction: 0,
                timestamp_precision: TimestampPrecision::Microseconds,
                captured_length: 42,
                original_length: 42,
            },
            raw_bytes: Vec::new(),
            link: LinkLayerSummary::Ethernet(EthernetFrameSummary {
                source_mac: "00:11:22:33:44:55".to_string(),
                destination_mac: "66:77:88:99:aa:bb".to_string(),
                ether_type: 0x0800,
            }),
            network: Some(NetworkLayerSummary::Ipv4(
                session_model::Ipv4PacketSummary {
                    source_ip: "10.0.0.1".to_string(),
                    destination_ip: "93.184.216.34".to_string(),
                    protocol: 6,
                    ttl: 64,
                    header_length: 20,
                    total_length: 42,
                },
            )),
            transport: Some(TransportLayerSummary::Tcp(
                session_model::TcpSegmentSummary {
                    source_port: 50000,
                    destination_port: 80,
                    sequence_number: 1,
                    acknowledgement_number: 0,
                    flags: 0x18,
                },
            )),
            application: Some(ApplicationLayerSummary::Http(
                session_model::HttpMessageSummary {
                    kind: "request".to_string(),
                    method: Some("GET".to_string()),
                    path: Some("/hello".to_string()),
                    status_code: None,
                    reason_phrase: None,
                    host: Some("example.com".to_string()),
                },
            )),
            fields: Vec::<FieldNode>::new(),
            notes: Vec::new(),
        }
    }

    fn sample_tls_packet() -> DecodedPacket {
        DecodedPacket {
            summary: PacketSummary {
                index: 2,
                timestamp_seconds: 0,
                timestamp_fraction: 0,
                timestamp_precision: TimestampPrecision::Microseconds,
                captured_length: 42,
                original_length: 42,
            },
            raw_bytes: Vec::new(),
            link: LinkLayerSummary::Ethernet(EthernetFrameSummary {
                source_mac: "00:11:22:33:44:55".to_string(),
                destination_mac: "66:77:88:99:aa:bb".to_string(),
                ether_type: 0x0800,
            }),
            network: Some(NetworkLayerSummary::Ipv4(
                session_model::Ipv4PacketSummary {
                    source_ip: "10.0.0.1".to_string(),
                    destination_ip: "93.184.216.34".to_string(),
                    protocol: 6,
                    ttl: 64,
                    header_length: 20,
                    total_length: 42,
                },
            )),
            transport: Some(TransportLayerSummary::Tcp(
                session_model::TcpSegmentSummary {
                    source_port: 50000,
                    destination_port: 443,
                    sequence_number: 1,
                    acknowledgement_number: 0,
                    flags: 0x18,
                },
            )),
            application: Some(ApplicationLayerSummary::TlsHandshake(
                session_model::TlsHandshakeSummary {
                    record_version: "3.3".to_string(),
                    handshake_type: "client_hello".to_string(),
                    handshake_length: 63,
                    server_name: Some("example.com".to_string()),
                },
            )),
            fields: Vec::<FieldNode>::new(),
            notes: Vec::new(),
        }
    }

    fn sample_http_response_packet() -> DecodedPacket {
        DecodedPacket {
            summary: PacketSummary {
                index: 3,
                timestamp_seconds: 0,
                timestamp_fraction: 0,
                timestamp_precision: TimestampPrecision::Microseconds,
                captured_length: 42,
                original_length: 42,
            },
            raw_bytes: Vec::new(),
            link: LinkLayerSummary::Ethernet(EthernetFrameSummary {
                source_mac: "66:77:88:99:aa:bb".to_string(),
                destination_mac: "00:11:22:33:44:55".to_string(),
                ether_type: 0x0800,
            }),
            network: Some(NetworkLayerSummary::Ipv4(
                session_model::Ipv4PacketSummary {
                    source_ip: "93.184.216.34".to_string(),
                    destination_ip: "10.0.0.1".to_string(),
                    protocol: 6,
                    ttl: 64,
                    header_length: 20,
                    total_length: 42,
                },
            )),
            transport: Some(TransportLayerSummary::Tcp(
                session_model::TcpSegmentSummary {
                    source_port: 80,
                    destination_port: 50000,
                    sequence_number: 2,
                    acknowledgement_number: 1,
                    flags: 0x18,
                },
            )),
            application: Some(ApplicationLayerSummary::Http(
                session_model::HttpMessageSummary {
                    kind: "response".to_string(),
                    method: None,
                    path: None,
                    status_code: Some(204),
                    reason_phrase: Some("No Content".to_string()),
                    host: None,
                },
            )),
            fields: Vec::<FieldNode>::new(),
            notes: Vec::new(),
        }
    }

    fn sample_stream_row() -> StreamRow {
        StreamRow {
            service: "tls".to_string(),
            protocol: "tcp".to_string(),
            client: "10.0.0.1:50000".to_string(),
            server: "93.184.216.34:443".to_string(),
            packets: 4,
            syn_packets: 0,
            fin_packets: 0,
            rst_packets: 1,
            session_state: "reset".to_string(),
            client_to_server_packets: 2,
            server_to_client_packets: 2,
            request_count: 1,
            response_count: 1,
            matched_transactions: 1,
            unmatched_requests: 0,
            unmatched_responses: 0,
            tls_client_hellos: 1,
            tls_server_hellos: 1,
            tls_certificates: 0,
            tls_finished_messages: 0,
            tls_handshake_cycles: 1,
            tls_incomplete_handshakes: 0,
            tls_handshake_state: "alert_seen".to_string(),
            tls_alert_count: 1,
            tls_alerts: vec!["fatal:handshake_failure".to_string()],
            total_captured_bytes: 300,
            first_packet_index: 1,
            last_packet_index: 4,
            transaction_timeline: vec!["1: client_hello -> server_hello + alert".to_string()],
            notes: vec!["reset_after_client_hello".to_string()],
        }
    }

    fn sample_pipelined_stream_row() -> StreamRow {
        StreamRow {
            service: "http".to_string(),
            protocol: "tcp".to_string(),
            client: "10.0.0.1:50000".to_string(),
            server: "93.184.216.34:80".to_string(),
            packets: 4,
            syn_packets: 0,
            fin_packets: 0,
            rst_packets: 0,
            session_state: "open".to_string(),
            client_to_server_packets: 2,
            server_to_client_packets: 2,
            request_count: 2,
            response_count: 2,
            matched_transactions: 2,
            unmatched_requests: 0,
            unmatched_responses: 0,
            tls_client_hellos: 0,
            tls_server_hellos: 0,
            tls_certificates: 0,
            tls_finished_messages: 0,
            tls_handshake_cycles: 0,
            tls_incomplete_handshakes: 0,
            tls_handshake_state: "not_applicable".to_string(),
            tls_alert_count: 0,
            tls_alerts: Vec::new(),
            total_captured_bytes: 512,
            first_packet_index: 10,
            last_packet_index: 13,
            transaction_timeline: vec![
                "1: GET /one".to_string(),
                "2: GET /two".to_string(),
                "3: 200 OK".to_string(),
                "4: 204 No Content".to_string(),
            ],
            notes: vec![
                "http stream shows pipelined requests before responses arrived.".to_string(),
                "client tcp stream contained out-of-order segments that were reordered during reassembly."
                    .to_string(),
            ],
        }
    }

    fn sample_transaction_row() -> TransactionRow {
        TransactionRow {
            service: "http".to_string(),
            protocol: "tcp".to_string(),
            client: "10.0.0.1:50000".to_string(),
            server: "93.184.216.34:80".to_string(),
            sequence: 1,
            request_summary: "GET /hello".to_string(),
            request_details: vec![
                transaction_detail("method", "GET"),
                transaction_detail("path", "/hello"),
                transaction_detail("host", "example.com"),
            ],
            response_summary: "200 OK".to_string(),
            response_details: vec![
                transaction_detail("status_code", "200"),
                transaction_detail("reason_phrase", "OK"),
                transaction_detail("body_bytes", "5"),
            ],
            state: "matched".to_string(),
            notes: vec!["clean_reassembly".to_string()],
        }
    }

    fn sample_tls_transaction_row() -> TransactionRow {
        TransactionRow {
            service: "tls".to_string(),
            protocol: "tcp".to_string(),
            client: "10.0.0.1:50000".to_string(),
            server: "93.184.216.34:443".to_string(),
            sequence: 1,
            request_summary: "client_hello".to_string(),
            request_details: vec![
                transaction_detail("record_version", "3.3"),
                transaction_detail("server_name", "example.com"),
                transaction_detail("alpn", "h2"),
                transaction_detail("handshake_messages", "client_hello"),
            ],
            response_summary: "server_hello + alert".to_string(),
            response_details: vec![
                transaction_detail("record_version", "3.3"),
                transaction_detail("handshake_messages", "server_hello,alert"),
                transaction_detail("alerts", "fatal:handshake_failure"),
                transaction_detail("certificate_messages", "0"),
            ],
            state: "alert_seen".to_string(),
            notes: vec!["alert_seen".to_string()],
        }
    }

    fn transaction_detail(key: &str, value: &str) -> TransactionDetail {
        TransactionDetail {
            key: key.to_string(),
            value: value.to_string(),
        }
    }
}
