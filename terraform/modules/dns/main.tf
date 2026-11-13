# Looks up an existing public hosted zone (registering the domain / delegating
# NS is a one-time manual step, so it isn't managed here).
data "aws_route53_zone" "this" {
  name         = var.zone_name
  private_zone = false
}

locals {
  # "@" means the zone apex; anything else is a subdomain of the zone.
  fqdns = {
    for n in var.record_names :
    n => n == "@" ? var.zone_name : "${n}.${var.zone_name}"
  }
}

resource "aws_route53_record" "host" {
  for_each = local.fqdns

  zone_id = data.aws_route53_zone.this.zone_id
  name    = each.value
  type    = "A"
  ttl     = var.ttl
  records = [var.ip_address]
}
