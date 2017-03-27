extern crate thanks;

extern crate diesel;

extern crate dotenv;

extern crate futures;

extern crate handlebars;

extern crate reqwest;

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use diesel::pg::PgConnection;
use diesel::prelude::*;
use reqwest::Url;

#[macro_use]
extern crate slog;
extern crate slog_term;

use slog::DrainExt;

use thanks::models::Project;
use thanks::mailmap::Mailmap;
use thanks::authors::AuthorStore;

#[derive(Debug,Deserialize)]
struct GitHubResponse(Vec<Object>);

#[derive(Debug,Deserialize)]
struct Object {
    sha: String,
    commit: Commit,
}

#[derive(Debug,Deserialize)]
struct Commit {
    author: Author,
}

#[derive(Debug,Deserialize)]
struct Author {
    name: String,
    email: String,
}

fn update_commit_db(log: &slog::Logger, project: &Project, lookup: &mut AuthorStore, connection: &PgConnection) {
    use thanks::schema::releases::dsl::*;
    use thanks::models::Release;
    use thanks::schema::commits::dsl::*;
    use thanks::models::Commit;
    use diesel::expression::dsl::any;

    let api_link = Url::parse(format!("https://api.github.com/repos/{}/commits", project.github_name).as_str()).unwrap();
    let mut resp = reqwest::get(api_link).unwrap();

    let response: GitHubResponse = resp.json().unwrap();

    // find the master release so we can assign commits to it
    let master_release = releases
        .filter(project_id.eq(project.id))
        .filter(version.eq("master"))
        .first::<Release>(connection)
        .expect("could not find release");

    let release_ids: Vec<i32> = Release::belonging_to(project).load::<Release>(connection).unwrap()
        .iter().map(|ref release| release.id).collect();

    for object in response.0 {
        info!(log, "Found commit with sha {}", object.sha);

        match commits
            .filter(release_id.eq(any(&release_ids)))
            .filter(sha.eq(&object.sha))
            .first::<Commit>(connection) {
            Ok(commit) => {
                info!(log, "Commit {} already in db, skipping", commit.sha);
                continue;
            },
            Err(_) => {
                info!(log, "Creating commit {} for release {}", object.sha, master_release.version);
                {
                    let author = lookup.get(&object.commit.author.name, &object.commit.author.email);
                    // this commit will be part of master
                    drop(thanks::commits::create(connection, &object.sha, &author, &master_release));
                }
            },
        };
    }
}

fn main() {
    let log = slog::Logger::root(slog_term::streamer().full().build().fuse(), o!("version" => env!("CARGO_PKG_VERSION")));

    use thanks::schema::projects::dsl::*;

    let connection = thanks::establish_connection();
    let mut lookup = AuthorStore::new(&connection, Mailmap::new(""));

    let projects_to_update: Vec<Project> = projects.load(&connection).expect("No projects found");
    for project in projects_to_update {
        info!(log, "Updating {}", project.name);
        update_commit_db(&log, &project, &mut lookup, &connection)
    }
}
