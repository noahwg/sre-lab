resource "aws_vpc" "this" {
  cidr_block           = var.vpc_cidr
  enable_dns_support   = true
  enable_dns_hostnames = true

  tags = { Name = "${var.name}-vpc" }
}

resource "aws_internet_gateway" "this" {
  vpc_id = aws_vpc.this.id

  tags = { Name = "${var.name}-igw" }
}

# Single public subnet: one host, no NAT gateway (saves ~$32/mo). The instance
# gets an Elastic IP in the compute module, so SSM reaches AWS over the IGW and
# no SSM VPC endpoints are needed.
resource "aws_subnet" "public" {
  vpc_id                  = aws_vpc.this.id
  cidr_block              = var.public_subnet_cidr
  availability_zone       = var.availability_zone
  map_public_ip_on_launch = false

  tags = { Name = "${var.name}-public-${var.availability_zone}" }
}

resource "aws_route_table" "public" {
  vpc_id = aws_vpc.this.id

  route {
    cidr_block = "0.0.0.0/0"
    gateway_id = aws_internet_gateway.this.id
  }

  tags = { Name = "${var.name}-public" }
}

resource "aws_route_table_association" "public" {
  subnet_id      = aws_subnet.public.id
  route_table_id = aws_route_table.public.id
}

# Management is SSM-only: there is deliberately NO port 22 rule. SSM Session
# Manager needs only outbound 443, which the egress rule below covers.
resource "aws_security_group" "host" {
  name_prefix = "${var.name}-host-"
  description = "URL shortener host: public web only, management via SSM (no SSH)"
  vpc_id      = aws_vpc.this.id

  tags = { Name = "${var.name}-host" }

  lifecycle {
    create_before_destroy = true
  }
}

resource "aws_vpc_security_group_ingress_rule" "http" {
  for_each = toset(var.web_ingress_cidrs)

  security_group_id = aws_security_group.host.id
  description       = "HTTP"
  ip_protocol       = "tcp"
  from_port         = 80
  to_port           = 80
  cidr_ipv4         = each.value
}

resource "aws_vpc_security_group_ingress_rule" "https" {
  for_each = toset(var.web_ingress_cidrs)

  security_group_id = aws_security_group.host.id
  description       = "HTTPS"
  ip_protocol       = "tcp"
  from_port         = 443
  to_port           = 443
  cidr_ipv4         = each.value
}

resource "aws_vpc_security_group_egress_rule" "all" {
  security_group_id = aws_security_group.host.id
  description       = "All outbound (SSM, package repos, ACME, image pulls)"
  ip_protocol       = "-1"
  cidr_ipv4         = "0.0.0.0/0"
}
