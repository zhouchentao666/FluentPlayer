use crate::dlna::types::{ControlService, ParsedDeviceDescription, PositionInfo};
use reqwest::Url;

const DEVICE_TYPE_PREFIX: &str = "urn:schemas-upnp-org:device:MediaRenderer:";
const AV_TRANSPORT_PREFIX: &str = "urn:schemas-upnp-org:service:AVTransport:";
const RENDERING_CONTROL_PREFIX: &str = "urn:schemas-upnp-org:service:RenderingControl:";
const SOAP_ENVELOPE_NAMESPACE: &str = "http://schemas.xmlsoap.org/soap/envelope/";

pub(crate) fn parse_device_description(
    document: &str,
    description_url: &str,
) -> Result<ParsedDeviceDescription, String> {
    let document = roxmltree::Document::parse(document)
        .map_err(|error| format!("DLNA 设备描述 XML 无效: {error}"))?;
    let description_url = parse_http_url(description_url)?;
    let root = document.root_element();
    let device =
        direct_child(root, "device").ok_or_else(|| "DLNA 设备描述缺少根设备".to_string())?;
    let device_type = child_text(device, "deviceType")
        .ok_or_else(|| "DLNA 设备描述缺少 deviceType".to_string())?;
    if !is_versioned_urn(&device_type, DEVICE_TYPE_PREFIX) {
        return Err("DLNA 设备不是 MediaRenderer".to_string());
    }
    let base_url = child_text(root, "URLBase")
        .map(|value| resolve_url(&description_url, &value))
        .transpose()?
        .unwrap_or(description_url);
    let service_list = direct_child(device, "serviceList")
        .ok_or_else(|| "DLNA 设备描述缺少 renderer serviceList".to_string())?;
    let av_transport = find_control_service(service_list, &base_url, AV_TRANSPORT_PREFIX)?;
    let rendering_control =
        find_control_service(service_list, &base_url, RENDERING_CONTROL_PREFIX)?;
    let name =
        child_text(device, "friendlyName").unwrap_or_else(|| "DLNA Media Renderer".to_string());

    Ok(ParsedDeviceDescription {
        name,
        av_transport,
        rendering_control,
    })
}

pub(crate) fn parse_position_info(
    document: &str,
    service_type: &str,
) -> Result<PositionInfo, String> {
    let document = roxmltree::Document::parse(document)
        .map_err(|error| format!("DLNA 位置响应 XML 无效: {error}"))?;
    let envelope = document.root_element();
    if !is_named(envelope, "Envelope")
        || envelope.tag_name().namespace() != Some(SOAP_ENVELOPE_NAMESPACE)
    {
        return Err("DLNA 位置响应不是 SOAP Envelope".to_string());
    }
    let body =
        direct_child(envelope, "Body").ok_or_else(|| "DLNA 位置响应缺少 SOAP Body".to_string())?;
    if body.tag_name().namespace() != Some(SOAP_ENVELOPE_NAMESPACE) {
        return Err("DLNA 位置响应 SOAP Body 命名空间无效".to_string());
    }
    let response = body
        .children()
        .find(|node| {
            is_named(*node, "GetPositionInfoResponse")
                && node.tag_name().namespace() == Some(service_type)
        })
        .ok_or_else(|| "DLNA 位置响应不是 GetPositionInfoResponse".to_string())?;
    let position =
        child_text(response, "RelTime").ok_or_else(|| "DLNA 位置响应缺少 RelTime".to_string())?;
    let duration = child_text(response, "TrackDuration")
        .ok_or_else(|| "DLNA 位置响应缺少 TrackDuration".to_string())?;
    Ok(PositionInfo {
        position: parse_duration(&position)?,
        duration: parse_duration(&duration)?,
    })
}

pub(crate) fn parse_soap_fault(document: &str) -> Option<String> {
    let document = roxmltree::Document::parse(document).ok()?;
    let envelope = document.root_element();
    if !is_named(envelope, "Envelope")
        || envelope.tag_name().namespace() != Some(SOAP_ENVELOPE_NAMESPACE)
    {
        return None;
    }
    let body = direct_child(envelope, "Body")?;
    let fault = body.children().find(|node| {
        is_named(*node, "Fault") && node.tag_name().namespace() == Some(SOAP_ENVELOPE_NAMESPACE)
    })?;
    let code = descendant_text(fault, "errorCode")
        .or_else(|| child_text(fault, "faultcode"))
        .unwrap_or_else(|| "unknown".to_string());
    let message = descendant_text(fault, "errorDescription")
        .or_else(|| child_text(fault, "faultstring"))
        .unwrap_or_else(|| "Unknown DLNA SOAP fault".to_string());
    Some(format!("DLNA SOAP Fault {code}: {message}"))
}

pub(crate) fn build_didl_metadata(title: &str, resource_url: &str) -> String {
    let title = escape_xml(if title.trim().is_empty() {
        "Mio Music"
    } else {
        title
    });
    let resource_url = escape_xml(resource_url);
    let mime = infer_audio_mime(resource_url.as_str());
    format!(
        "<DIDL-Lite xmlns=\"urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\" xmlns:upnp=\"urn:schemas-upnp-org:metadata-1-0/upnp/\"><item id=\"0\" parentID=\"0\" restricted=\"1\"><dc:title>{title}</dc:title><upnp:class>object.item.audioItem.musicTrack</upnp:class><res protocolInfo=\"http-get:*:{mime}:*\">{resource_url}</res></item></DIDL-Lite>"
    )
}

pub(crate) fn build_soap_envelope(
    service: &str,
    action: &str,
    values: &[(&str, String)],
) -> String {
    let values = values
        .iter()
        .map(|(name, value)| format!("<{name}>{}</{name}>", escape_xml(value)))
        .collect::<String>();
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?><s:Envelope xmlns:s=\"http://schemas.xmlsoap.org/soap/envelope/\" s:encodingStyle=\"http://schemas.xmlsoap.org/soap/encoding/\"><s:Body><u:{action} xmlns:u=\"{service}\">{values}</u:{action}></s:Body></s:Envelope>"
    )
}

pub(crate) fn format_duration(seconds: f64) -> String {
    let seconds = if seconds.is_finite() && seconds > 0.0 {
        seconds.floor() as u64
    } else {
        0
    };
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        (seconds % 3600) / 60,
        seconds % 60
    )
}

fn find_control_service(
    service_list: roxmltree::Node<'_, '_>,
    base_url: &Url,
    service_prefix: &str,
) -> Result<ControlService, String> {
    let service = service_list.children().find(|service| {
        is_named(*service, "service")
            && child_text(*service, "serviceType")
                .is_some_and(|service_type| is_versioned_urn(&service_type, service_prefix))
    });
    let service = service.ok_or_else(|| format!("DLNA 设备不支持 {} 服务", service_prefix))?;
    let service_type = child_text(service, "serviceType").unwrap();
    let control_url = child_text(service, "controlURL")
        .ok_or_else(|| format!("DLNA {} 服务缺少 controlURL", service_prefix))?;
    Ok(ControlService {
        url: resolve_url(base_url, &control_url)?,
        service_type,
    })
}

fn infer_audio_mime(resource_url: &str) -> &'static str {
    let path = resource_url
        .split('?')
        .next()
        .unwrap_or(resource_url)
        .to_ascii_lowercase();
    if path.ends_with(".mp3") {
        "audio/mpeg"
    } else if path.ends_with(".flac") {
        "audio/flac"
    } else if path.ends_with(".aac") {
        "audio/aac"
    } else if path.ends_with(".m4a") {
        "audio/mp4"
    } else if path.ends_with(".ogg") || path.ends_with(".opus") {
        "audio/ogg"
    } else if path.ends_with(".wav") {
        "audio/wav"
    } else {
        "audio/*"
    }
}

fn direct_child<'a>(node: roxmltree::Node<'a, 'a>, name: &str) -> Option<roxmltree::Node<'a, 'a>> {
    node.children().find(|child| is_named(*child, name))
}

fn child_text(node: roxmltree::Node<'_, '_>, name: &str) -> Option<String> {
    direct_child(node, name)
        .and_then(|child| child.text())
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToString::to_string)
}

fn descendant_text(node: roxmltree::Node<'_, '_>, name: &str) -> Option<String> {
    node.descendants()
        .find(|child| is_named(*child, name))
        .and_then(|child| child.text())
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToString::to_string)
}

fn is_versioned_urn(value: &str, prefix: &str) -> bool {
    value
        .strip_prefix(prefix)
        .is_some_and(|version| version.parse::<u32>().is_ok_and(|version| version > 0))
}

fn is_named(node: roxmltree::Node<'_, '_>, name: &str) -> bool {
    node.is_element() && node.tag_name().name().eq_ignore_ascii_case(name)
}

fn parse_duration(value: &str) -> Result<f64, String> {
    let parts = value.trim().split(':').collect::<Vec<_>>();
    if parts.len() != 3 {
        return Err(format!("DLNA 时间格式无效: {value}"));
    }
    let parsed = parts
        .iter()
        .map(|part| {
            part.parse::<f64>()
                .map_err(|_| format!("DLNA 时间格式无效: {value}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if parsed.iter().any(|part| !part.is_finite()) {
        return Err(format!("DLNA 时间格式无效: {value}"));
    }
    Ok(parsed[0] * 3600.0 + parsed[1] * 60.0 + parsed[2])
}

fn resolve_url(base_url: &Url, raw_url: &str) -> Result<Url, String> {
    Url::parse(raw_url)
        .or_else(|_| base_url.join(raw_url))
        .map_err(|error| format!("DLNA 服务地址无效: {error}"))
        .and_then(|url| parse_http_url(url.as_str()))
}

fn parse_http_url(raw_url: &str) -> Result<Url, String> {
    let url = Url::parse(raw_url).map_err(|error| format!("DLNA 地址无效: {error}"))?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("DLNA 地址必须是无凭据的 HTTP(S) 地址".to_string());
    }
    Ok(url)
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::{
        build_didl_metadata, build_soap_envelope, format_duration, parse_device_description,
        parse_position_info, parse_soap_fault,
    };

    const DESCRIPTION_XML: &str = r#"<root xmlns="urn:schemas-upnp-org:device-1-0"><URLBase>http://192.168.1.20:1400</URLBase><device><deviceType>urn:schemas-upnp-org:device:MediaRenderer:2</deviceType><friendlyName>Living Room</friendlyName><serviceList><service><serviceType>urn:schemas-upnp-org:service:AVTransport:1</serviceType><controlURL>/MediaRenderer/AVTransport/Control</controlURL></service><service><serviceType>urn:schemas-upnp-org:service:RenderingControl:1</serviceType><controlURL>/MediaRenderer/RenderingControl/Control</controlURL></service></serviceList></device></root>"#;
    const POSITION_XML: &str = r#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body><u:GetPositionInfoResponse xmlns:u="urn:schemas-upnp-org:service:AVTransport:1"><TrackDuration>00:04:05</TrackDuration><RelTime>00:01:02</RelTime></u:GetPositionInfoResponse></s:Body></s:Envelope>"#;

    #[test]
    fn parses_media_renderer_services() {
        let device =
            parse_device_description(DESCRIPTION_XML, "http://192.168.1.20:1400/xml/device.xml")
                .unwrap();
        assert_eq!(device.name, "Living Room");
        assert_eq!(
            device.av_transport.url.as_str(),
            "http://192.168.1.20:1400/MediaRenderer/AVTransport/Control"
        );
    }

    #[test]
    fn parses_position_info() {
        let position =
            parse_position_info(POSITION_XML, "urn:schemas-upnp-org:service:AVTransport:1")
                .unwrap();
        assert_eq!((position.position, position.duration), (62.0, 245.0));
    }

    #[test]
    fn parses_soap_fault_code_and_message() {
        let fault = r#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body><s:Fault><faultcode>s:Client</faultcode><faultstring>UPnPError</faultstring><detail><UPnPError><errorCode>701</errorCode><errorDescription>Transition not available</errorDescription></UPnPError></detail></s:Fault></s:Body></s:Envelope>"#;

        assert_eq!(
            parse_soap_fault(fault).as_deref(),
            Some("DLNA SOAP Fault 701: Transition not available")
        );
    }

    #[test]
    fn rejects_non_renderer_nested_services_and_wrong_position_scope() {
        assert!(parse_device_description(
            &DESCRIPTION_XML.replace("MediaRenderer:2", "MediaServer:1"),
            "http://192.168.1.20/device.xml"
        )
        .is_err());
        let nested = DESCRIPTION_XML
            .replace("<serviceList>", "<deviceList><device><serviceList>")
            .replace("</serviceList>", "</serviceList></device></deviceList>");
        assert!(parse_device_description(&nested, "http://192.168.1.20/device.xml").is_err());
        let wrong = POSITION_XML.replace("GetPositionInfoResponse", "OtherResponse");
        assert!(parse_position_info(&wrong, "urn:schemas-upnp-org:service:AVTransport:1").is_err());
        let wrong_namespace = POSITION_XML.replace(
            "http://schemas.xmlsoap.org/soap/envelope/",
            "urn:unexpected:soap",
        );
        assert!(parse_position_info(
            &wrong_namespace,
            "urn:schemas-upnp-org:service:AVTransport:1"
        )
        .is_err());
    }

    #[test]
    fn preserves_service_version_and_escapes_audio_metadata() {
        let device = parse_device_description(
            &DESCRIPTION_XML.replace("AVTransport:1", "AVTransport:2"),
            "http://192.168.1.20/device.xml",
        )
        .unwrap();
        assert!(
            build_soap_envelope(&device.av_transport.service_type, "Play", &[])
                .contains("AVTransport:2")
        );
        assert_eq!(format_duration(62.9), "00:01:02");
        assert!(build_didl_metadata("A & B", "https://example.com/a?x=1&y=2").contains("audio/*"));
        assert!(
            build_didl_metadata("Track", "https://example.com/track.flac").contains("audio/flac")
        );
        assert!(
            build_didl_metadata("Track", "https://example.com/track.aac").contains("audio/aac")
        );
        assert!(
            build_didl_metadata("Track", "https://example.com/track.m4a").contains("audio/mp4")
        );
    }
}
