data "aws_availability_zones" "available" {
  state = "available"
}

module "network" {
  source = "../../../modules/network"

  name              = var.name
  availability_zone = data.aws_availability_zones.available.names[0]
  web_ingress_cidrs = var.web_ingress_cidrs
}
