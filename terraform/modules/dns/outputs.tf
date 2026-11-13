output "fqdns" {
  description = "Map of record_name => fully qualified domain name."
  value       = local.fqdns
}

output "zone_id" {
  value = data.aws_route53_zone.this.zone_id
}
