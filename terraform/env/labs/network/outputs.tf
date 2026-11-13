output "vpc_id" {
  value = module.network.vpc_id
}

output "subnet_id" {
  value = module.network.subnet_id
}

output "availability_zone" {
  value = module.network.availability_zone
}

output "security_group_id" {
  value = module.network.security_group_id
}
