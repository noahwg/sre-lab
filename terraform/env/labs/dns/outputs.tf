output "fqdns" {
  value = module.dns.fqdns
}

output "primary_fqdn" {
  value = module.dns.fqdns[var.record_names[0]]
}
