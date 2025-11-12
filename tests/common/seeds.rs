use anyhow::Result;
use leaf::oracle::OracleClient;

pub async fn create_source_objects(conn: &OracleClient) -> Result<()> {
    let scripts = vec![
        // === Schema ===
        r#"
        CREATE USER schema1 IDENTIFIED BY schema1_password
        "#,
        r#"
        GRANT CONNECT, RESOURCE TO schema1
        "#,
        r#"
        CREATE USER schema2 IDENTIFIED BY schema2_password
        "#,
        r#"
        GRANT CONNECT, RESOURCE TO schema2
        "#,
        // === Tables ===
        r#"
        CREATE TABLE schema1.emp (id NUMBER PRIMARY KEY)
        "#,
        r#"
        CREATE TABLE schema1.dept (dept_id NUMBER PRIMARY KEY)
        "#,
        r#"
        CREATE TABLE schema2.emp (id NUMBER PRIMARY KEY)
        "#,
        r#"
        CREATE TABLE schema2.salary (salary_id NUMBER PRIMARY KEY)
        "#,
        // === Views ===
        r#"
        CREATE OR REPLACE VIEW schema1.v_emp AS
        SELECT id FROM schema1.emp
        "#,
        r#"
        CREATE OR REPLACE VIEW schema2.v_salary AS
        SELECT salary_id FROM schema2.salary
        "#,
        // === Types ===
        r#"
        CREATE OR REPLACE TYPE schema1.address_t AS OBJECT (
            street VARCHAR2(50),
            city   VARCHAR2(50)
        )
        "#,
        r#"
        CREATE OR REPLACE TYPE schema2.department_t AS OBJECT (
            dept_name VARCHAR2(50)
        )
        "#,
        // === Procedures ===
        r#"
        CREATE OR REPLACE PROCEDURE schema1.add_emp(p_id NUMBER) AS
        BEGIN
            INSERT INTO schema1.emp(id) VALUES (p_id)
        END
        "#,
        r#"
        CREATE OR REPLACE PROCEDURE schema2.add_salary(p_salary_id NUMBER) AS
        BEGIN
            INSERT INTO schema2.salary(salary_id) VALUES (p_salary_id)
        END
        "#,
        // === Packages ===
        r#"
        CREATE OR REPLACE PACKAGE schema1.emp_pkg AS
            PROCEDURE log_emp(p_id NUMBER)
        END emp_pkg
        /
        CREATE OR REPLACE PACKAGE BODY schema1.emp_pkg AS
            PROCEDURE log_emp(p_id NUMBER) IS
            BEGIN
                DBMS_OUTPUT.PUT_LINE('Emp ID: ' || p_id)
            END
        END emp_pkg
        "#,
        r#"
        CREATE OR REPLACE PACKAGE schema2.salary_pkg AS
            PROCEDURE log_salary(p_id NUMBER)
        END salary_pkg
        /
        CREATE OR REPLACE PACKAGE BODY schema2.salary_pkg AS
            PROCEDURE log_salary(p_id NUMBER) IS
            BEGIN
                DBMS_OUTPUT.PUT_LINE('Salary ID: ' || p_id)
            END
        END salary_pkg
        "#,
    ];

    for sql in scripts {
        conn.execute(sql).await?
    }

    Ok(())
}

pub async fn create_target_objects(conn: &OracleClient) -> Result<()> {
    let scripts = vec![
        // === Schema ===
        r#"
        CREATE USER schema1 IDENTIFIED BY schema1_password_tgt
        "#,
        r#"
        GRANT CONNECT, RESOURCE TO schema1
        "#,
        r#"
        CREATE USER schema2 IDENTIFIED BY schema2_password_tgt
        "#,
        r#"
        GRANT CONNECT, RESOURCE TO schema2
        "#,
        // === Tables ===
        r#"
        CREATE TABLE schema1.emp (id NUMBER PRIMARY KEY, name VARCHAR2(50))
        "#,
        r#"
        CREATE TABLE schema1.dept (dept_id NUMBER PRIMARY KEY)
        "#,
        r#"
        CREATE TABLE schema2.salary (salary_id NUMBER PRIMARY KEY, amount NUMBER)
        "#,
        r#"
        CREATE TABLE schema2.bonus (bonus_id NUMBER PRIMARY KEY)
        "#,
        // === Views ===
        r#"
        CREATE OR REPLACE VIEW schema1.v_emp AS
        SELECT id, name FROM schema1.emp
        "#,
        r#"
        CREATE OR REPLACE VIEW schema2.v_bonus AS
        SELECT bonus_id FROM schema2.bonus
        "#,
        // === Types ===
        r#"
        CREATE OR REPLACE TYPE schema1.address_t AS OBJECT (
            street VARCHAR2(100),
            city   VARCHAR2(100),
            zip    VARCHAR2(10)
        )
        "#,
        r#"
        CREATE OR REPLACE TYPE schema2.department_t AS OBJECT (
            dept_name VARCHAR2(50),
            manager   VARCHAR2(50)
        )
        "#,
        // === Procedures ===
        r#"
        CREATE OR REPLACE PROCEDURE schema1.add_emp(p_id NUMBER, p_name VARCHAR2) AS
        BEGIN
            INSERT INTO schema1.emp(id, name) VALUES (p_id, p_name)
        END
        "#,
        r#"
        CREATE OR REPLACE PROCEDURE schema2.add_bonus(p_bonus_id NUMBER) AS
        BEGIN
            INSERT INTO schema2.bonus(bonus_id) VALUES (p_bonus_id)
        END
        "#,
        // === Packages ===
        r#"
        CREATE OR REPLACE PACKAGE schema1.emp_pkg AS
            PROCEDURE log_emp(p_id NUMBER, p_name VARCHAR2)
        END emp_pkg
        /
        CREATE OR REPLACE PACKAGE BODY schema1.emp_pkg AS
            PROCEDURE log_emp(p_id NUMBER, p_name VARCHAR2) IS
            BEGIN
                DBMS_OUTPUT.PUT_LINE('Emp: ' || p_id || ' - ' || p_name)
            END
        END emp_pkg
        "#,
        r#"
        CREATE OR REPLACE PACKAGE schema2.salary_pkg AS
            PROCEDURE log_salary(p_id NUMBER, p_amount NUMBER)
        END salary_pkg
        /
        CREATE OR REPLACE PACKAGE BODY schema2.salary_pkg AS
            PROCEDURE log_salary(p_id NUMBER, p_amount NUMBER) IS
            BEGIN
                DBMS_OUTPUT.PUT_LINE('Salary ID: ' || p_id || ' Amount: ' || p_amount)
            END
        END salary_pkg
        "#,
    ];

    for sql in scripts {
        conn.execute(sql).await?
    }

    Ok(())
}

pub async fn drop_users(conn: &OracleClient) -> Result<()> {
    let scripts = vec![
        r#"
        DROP USER schema1 CASCADE
        "#,
        r#"
        DROP USER schema2 CASCADE
        "#,
    ];

    for sql in scripts {
        let res = conn.execute(sql).await;
        if let Err(e) = res {
            if !e.to_string().contains("ORA-01918") {
                return Err(e);
            }
        }
    }

    Ok(())
}

pub async fn init_source(conn: &OracleClient) -> Result<()> {
    create_source_objects(conn).await
}

pub async fn init_target(conn: &OracleClient) -> Result<()> {
    create_target_objects(conn).await
}

pub async fn cleanup(conn: &OracleClient) -> Result<()> {
    drop_users(conn).await
}
