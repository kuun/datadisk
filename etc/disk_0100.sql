drop table if exists disk_user;
drop table if exists disk_file_info;

create table disk_user
(
    username varchar(255) NOT NULL primary key unique,
    password varchar(255) NOT NULL,
    full_name varchar(255),
    phone varchar(255),
    email varchar(255),
    contacts_id bigint,
    status int NOT NULL default 1,
    last_login int
);
-- password value is sha256sum
-- password is admin123
insert into disk_user(username, password, full_name) values('admin', '240be518fabd2724ddb6f04eeb1da5967448d7e831c08c8fa822809f74c720a9', 'admin');

create table disk_contacts (
    id bigserial NOT NULL PRIMARY KEY,
    name varchar(255) NOT NULL,
    level int NOT NULL,
    parent_id bigint
);


create table disk_file_info
(
    id bigserial NOT NULL PRIMARY KEY,
    parent_id bigint,
    name varchar(255) NOT NULL,
    username varchar(255) NOT NULL,
    size int NOT NULL,
    type varchar(255) NOT NULL,
    create_time int NOT NULL,
    modify_time int
);

commit;