output "vpc_id" {
  value = aws_vpc.this.id
}

output "subnet_id" {
  value = aws_subnet.public.id
}

output "availability_zone" {
  value = aws_subnet.public.availability_zone
}

output "security_group_id" {
  value = aws_security_group.host.id
}
