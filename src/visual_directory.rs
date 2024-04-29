use anyhow::Result;
use lcr::data::VisualPerson;
use sheets::types::{
    BatchUpdateSpreadsheetRequest, CellData, CellFormat, Dimension, DimensionProperties,
    DimensionRange, GridRange, HorizontalAlignment, MergeCellsRequest, MergeType,
    RepeatCellRequest, Request, Spreadsheet, SpreadsheetProperties,
    UpdateDimensionPropertiesRequest, ValueRange, VerticalAlignment, WrapStrategy,
};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use url::Url;

// How many spreadsheet rows can fit on one piece of portrait paper.
// Depends on row height, which is determined by image height.
const NUM_ROWS_PER_PRINTED_SHEET: u32 = 11;

// How many columns of people are displayed horizontally across page.
const NUM_COLS_PER_PRINTED_SHEET: u32 = 3;

// How many spreadsheet columns per person column.
const NUM_COLS_PER_PERSON: u32 = 2; // Photo and name

pub async fn create_visual_directory(client: &mut lcr::client::Client) -> Result<()> {
    let visual_member_list = client.visual_member_list()?;
    const REDIRECT_URL: &str = "127.0.0.1:8080";
    let mut client = sheets::Client::new(
        std::env::var("GOOGLE_SHEETS_CLIENT_ID").expect("Couldn't read GOOGLE_SHEETS_CLIENT_ID"),
        std::env::var("GOOGLE_SHEETS_CLIENT_SECRET")
            .expect("Couldn't read GOOGLE_SHEETS_CLIENT_ID"),
        format!("http://{}", REDIRECT_URL),
        "",
        "",
    );
    let user_consent_url =
        client.user_consent_url(&["https://www.googleapis.com/auth/spreadsheets".to_string()]);

    println!("\nOpen in browser:\n\n{}\n", user_consent_url);

    let (code, state) = wait_for_redirect(REDIRECT_URL);
    client.get_access_token(&code, &state).await?;

    let spreadsheet = create_spreadsheet(&mut client).await?;
    populate_spreadsheet(
        &mut client,
        &spreadsheet.spreadsheet_id,
        &visual_member_list,
    )
    .await?;
    format_spreadsheet(
        &mut client,
        &spreadsheet.spreadsheet_id,
        visual_member_list.len(),
    )
    .await?;

    println!("Spreadsheet: {}", spreadsheet.spreadsheet_url);

    Ok(())
}

fn wait_for_redirect(redirect_url: &str) -> (String, String) {
    let mut code = String::new();
    let mut state = String::new();

    let listener = TcpListener::bind(redirect_url).unwrap();
    if let Some(mut stream) = listener.incoming().flatten().next() {
        let mut reader = BufReader::new(&stream);
        let mut request_line = String::new();
        reader.read_line(&mut request_line).unwrap();

        let redirect_url = request_line.split_whitespace().nth(1).unwrap();
        let url = Url::parse(&("http://localhost".to_string() + redirect_url)).unwrap();

        let code_pair = url
            .query_pairs()
            .find(|pair| {
                let (key, _) = pair;
                key == "code"
            })
            .unwrap();

        let (_, value) = code_pair;
        code = value.into_owned();

        let state_pair = url
            .query_pairs()
            .find(|pair| {
                let (key, _) = pair;
                key == "state"
            })
            .unwrap();

        let (_, value) = state_pair;
        state = value.into_owned();

        let message = "Go back to your terminal :)";
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-length: {}\r\n\r\n{}",
            message.len(),
            message
        );
        stream.write_all(response.as_bytes()).unwrap();
    }

    (code, state)
}

async fn create_spreadsheet(client: &mut sheets::Client) -> Result<Spreadsheet> {
    let spreadsheet = Spreadsheet {
        data_source_schedules: vec![],
        data_sources: vec![],
        developer_metadata: vec![],
        named_ranges: vec![],
        properties: Some(SpreadsheetProperties {
            auto_recalc: None,
            default_format: None,
            iterative_calculation_settings: None,
            locale: "en".to_string(),
            spreadsheet_theme: None,
            time_zone: "America/Los_Angeles".to_string(),
            title: "Photo Directory".to_string(),
        }),
        sheets: vec![],
        spreadsheet_id: "".to_string(),
        spreadsheet_url: "".to_string(),
    };

    Ok(client.spreadsheets().create(&spreadsheet).await?.body)
}

fn size_of_spreadsheet(num_members: usize) -> (u32, u32) {
    let num_columns =
        NUM_COLS_PER_PRINTED_SHEET * NUM_COLS_PER_PERSON + (NUM_COLS_PER_PRINTED_SHEET - 1);

    let num_full_printed_pages = (num_members as f64
        / ((NUM_COLS_PER_PRINTED_SHEET * NUM_ROWS_PER_PRINTED_SHEET) as f64))
        .floor() as u32;
    let mut num_rows = num_full_printed_pages * NUM_ROWS_PER_PRINTED_SHEET;

    let num_left = num_members as u32 - (num_rows * NUM_COLS_PER_PRINTED_SHEET);
    if num_left <= NUM_ROWS_PER_PRINTED_SHEET {
        num_rows += num_left;
    } else {
        num_rows += NUM_ROWS_PER_PRINTED_SHEET;
    }

    (num_rows, num_columns)
}

async fn populate_spreadsheet(
    client: &mut sheets::Client,
    spreadsheet_id: &str,
    members: &[VisualPerson],
) -> Result<()> {
    let (num_rows, num_columns) = size_of_spreadsheet(members.len());
    let end_col = char::from_u32(num_columns + 'A' as u32 - 1).unwrap();
    let range = format!("A1:{}{}", end_col, num_rows);
    let mut data = vec![vec!["".to_string(); num_columns as usize]; num_rows as usize];

    let mut x_off = 0;
    for sheet_people in
        members.chunks(NUM_ROWS_PER_PRINTED_SHEET as usize * NUM_COLS_PER_PRINTED_SHEET as usize)
    {
        for (i, member) in sheet_people.iter().enumerate() {
            let x = x_off + (i % NUM_ROWS_PER_PRINTED_SHEET as usize);
            let y = i / NUM_ROWS_PER_PRINTED_SHEET as usize * (NUM_COLS_PER_PERSON as usize + 1);

            data[x][y] = format!("=image(\"{}\")", member.photo_url);
            data[x][y + 1] = member.name.to_string();
        }

        x_off += NUM_ROWS_PER_PRINTED_SHEET as usize;
    }

    client
        .spreadsheets()
        .values_update(
            spreadsheet_id,
            &range,
            false,
            sheets::types::DateTimeRenderOption::FormattedString,
            sheets::types::ValueRenderOption::FormattedValue,
            sheets::types::ValueInputOption::UserEntered,
            &ValueRange {
                major_dimension: Some(Dimension::Rows),
                range: range.to_string(),
                values: data,
            },
        )
        .await?;

    Ok(())
}

async fn format_spreadsheet(
    client: &mut sheets::Client,
    spreadsheet_id: &str,
    num_members: usize,
) -> Result<()> {
    let (num_rows, num_columns) = size_of_spreadsheet(num_members);

    let horizontally_size_name_columns = (1..num_columns)
        .step_by(NUM_COLS_PER_PERSON as usize + 1)
        .map(|i| Request {
            add_banding: None,
            add_chart: None,
            add_conditional_format_rule: None,
            add_data_source: None,
            add_dimension_group: None,
            add_filter_view: None,
            add_named_range: None,
            add_protected_range: None,
            add_sheet: None,
            add_slicer: None,
            append_cells: None,
            append_dimension: None,
            auto_fill: None,
            auto_resize_dimensions: None,
            clear_basic_filter: None,
            copy_paste: None,
            create_developer_metadata: None,
            cut_paste: None,
            delete_banding: None,
            delete_conditional_format_rule: None,
            delete_data_source: None,
            delete_developer_metadata: None,
            delete_dimension: None,
            delete_dimension_group: None,
            delete_duplicates: None,
            delete_embedded_object: None,
            delete_filter_view: None,
            delete_named_range: None,
            delete_protected_range: None,
            delete_range: None,
            delete_sheet: None,
            duplicate_filter_view: None,
            duplicate_sheet: None,
            find_replace: None,
            insert_dimension: None,
            insert_range: None,
            merge_cells: None,
            move_dimension: None,
            paste_data: None,
            randomize_range: None,
            refresh_data_source: None,
            repeat_cell: None,
            set_basic_filter: None,
            set_data_validation: None,
            sort_range: None,
            text_to_columns: None,
            trim_whitespace: None,
            unmerge_cells: None,
            update_banding: None,
            update_borders: None,
            update_cells: None,
            update_chart_spec: None,
            update_conditional_format_rule: None,
            update_data_source: None,
            update_developer_metadata: None,
            update_dimension_group: None,
            update_dimension_properties: Some(UpdateDimensionPropertiesRequest {
                data_source_sheet_range: None,
                fields: "pixelSize".to_string(),
                properties: Some(DimensionProperties {
                    data_source_column_reference: None,
                    developer_metadata: vec![],
                    hidden_by_filter: false,
                    hidden_by_user: false,
                    pixel_size: 130,
                }),
                range: Some(DimensionRange {
                    dimension: Some(Dimension::Columns),
                    end_index: i as i64 + 1,
                    sheet_id: 0,
                    start_index: i as i64,
                }),
            }),
            update_embedded_object_border: None,
            update_embedded_object_position: None,
            update_filter_view: None,
            update_named_range: None,
            update_protected_range: None,
            update_sheet_properties: None,
            update_slicer_spec: None,
            update_spreadsheet_properties: None,
        });

    let vertically_size_rows = std::iter::once(Request {
        add_banding: None,
        add_chart: None,
        add_conditional_format_rule: None,
        add_data_source: None,
        add_dimension_group: None,
        add_filter_view: None,
        add_named_range: None,
        add_protected_range: None,
        add_sheet: None,
        add_slicer: None,
        append_cells: None,
        append_dimension: None,
        auto_fill: None,
        auto_resize_dimensions: None,
        clear_basic_filter: None,
        copy_paste: None,
        create_developer_metadata: None,
        cut_paste: None,
        delete_banding: None,
        delete_conditional_format_rule: None,
        delete_data_source: None,
        delete_developer_metadata: None,
        delete_dimension: None,
        delete_dimension_group: None,
        delete_duplicates: None,
        delete_embedded_object: None,
        delete_filter_view: None,
        delete_named_range: None,
        delete_protected_range: None,
        delete_range: None,
        delete_sheet: None,
        duplicate_filter_view: None,
        duplicate_sheet: None,
        find_replace: None,
        insert_dimension: None,
        insert_range: None,
        merge_cells: None,
        move_dimension: None,
        paste_data: None,
        randomize_range: None,
        refresh_data_source: None,
        repeat_cell: None,
        set_basic_filter: None,
        set_data_validation: None,
        sort_range: None,
        text_to_columns: None,
        trim_whitespace: None,
        unmerge_cells: None,
        update_banding: None,
        update_borders: None,
        update_cells: None,
        update_chart_spec: None,
        update_conditional_format_rule: None,
        update_data_source: None,
        update_developer_metadata: None,
        update_dimension_group: None,
        update_dimension_properties: Some(UpdateDimensionPropertiesRequest {
            data_source_sheet_range: None,
            fields: "pixelSize".to_string(),
            properties: Some(DimensionProperties {
                data_source_column_reference: None,
                developer_metadata: vec![],
                hidden_by_filter: false,
                hidden_by_user: false,
                pixel_size: 80,
            }),
            range: Some(DimensionRange {
                dimension: Some(Dimension::Rows),
                end_index: num_rows as i64,
                sheet_id: 0,
                start_index: 0,
            }),
        }),
        update_embedded_object_border: None,
        update_embedded_object_position: None,
        update_filter_view: None,
        update_named_range: None,
        update_protected_range: None,
        update_sheet_properties: None,
        update_slicer_spec: None,
        update_spreadsheet_properties: None,
    });

    let horizontally_size_photos_columns = (0..num_columns)
        .step_by(NUM_COLS_PER_PERSON as usize + 1)
        .map(|i| Request {
            add_banding: None,
            add_chart: None,
            add_conditional_format_rule: None,
            add_data_source: None,
            add_dimension_group: None,
            add_filter_view: None,
            add_named_range: None,
            add_protected_range: None,
            add_sheet: None,
            add_slicer: None,
            append_cells: None,
            append_dimension: None,
            auto_fill: None,
            auto_resize_dimensions: None,
            clear_basic_filter: None,
            copy_paste: None,
            create_developer_metadata: None,
            cut_paste: None,
            delete_banding: None,
            delete_conditional_format_rule: None,
            delete_data_source: None,
            delete_developer_metadata: None,
            delete_dimension: None,
            delete_dimension_group: None,
            delete_duplicates: None,
            delete_embedded_object: None,
            delete_filter_view: None,
            delete_named_range: None,
            delete_protected_range: None,
            delete_range: None,
            delete_sheet: None,
            duplicate_filter_view: None,
            duplicate_sheet: None,
            find_replace: None,
            insert_dimension: None,
            insert_range: None,
            merge_cells: None,
            move_dimension: None,
            paste_data: None,
            randomize_range: None,
            refresh_data_source: None,
            repeat_cell: None,
            set_basic_filter: None,
            set_data_validation: None,
            sort_range: None,
            text_to_columns: None,
            trim_whitespace: None,
            unmerge_cells: None,
            update_banding: None,
            update_borders: None,
            update_cells: None,
            update_chart_spec: None,
            update_conditional_format_rule: None,
            update_data_source: None,
            update_developer_metadata: None,
            update_dimension_group: None,
            update_dimension_properties: Some(UpdateDimensionPropertiesRequest {
                data_source_sheet_range: None,
                fields: "pixelSize".to_string(),
                properties: Some(DimensionProperties {
                    data_source_column_reference: None,
                    developer_metadata: vec![],
                    hidden_by_filter: false,
                    hidden_by_user: false,
                    pixel_size: 80,
                }),
                range: Some(DimensionRange {
                    dimension: Some(Dimension::Columns),
                    end_index: i as i64 + 1,
                    sheet_id: 0,
                    start_index: i as i64,
                }),
            }),
            update_embedded_object_border: None,
            update_embedded_object_position: None,
            update_filter_view: None,
            update_named_range: None,
            update_protected_range: None,
            update_sheet_properties: None,
            update_slicer_spec: None,
            update_spreadsheet_properties: None,
        });

    let vert_and_horiz_center_items_in_rows = std::iter::once(Request {
        add_banding: None,
        add_chart: None,
        add_conditional_format_rule: None,
        add_data_source: None,
        add_dimension_group: None,
        add_filter_view: None,
        add_named_range: None,
        add_protected_range: None,
        add_sheet: None,
        add_slicer: None,
        append_cells: None,
        append_dimension: None,
        auto_fill: None,
        auto_resize_dimensions: None,
        clear_basic_filter: None,
        copy_paste: None,
        create_developer_metadata: None,
        cut_paste: None,
        delete_banding: None,
        delete_conditional_format_rule: None,
        delete_data_source: None,
        delete_developer_metadata: None,
        delete_dimension: None,
        delete_dimension_group: None,
        delete_duplicates: None,
        delete_embedded_object: None,
        delete_filter_view: None,
        delete_named_range: None,
        delete_protected_range: None,
        delete_range: None,
        delete_sheet: None,
        duplicate_filter_view: None,
        duplicate_sheet: None,
        find_replace: None,
        insert_dimension: None,
        insert_range: None,
        merge_cells: None,
        move_dimension: None,
        paste_data: None,
        randomize_range: None,
        refresh_data_source: None,
        repeat_cell: Some(RepeatCellRequest {
            range: Some(GridRange {
                end_column_index: num_columns as i64,
                end_row_index: num_rows as i64,
                sheet_id: 0,
                start_column_index: 0,
                start_row_index: 0,
            }),
            cell: Some(CellData {
                data_source_formula: None,
                data_source_table: None,
                data_validation: None,
                effective_format: None,
                effective_value: None,
                formatted_value: "".to_string(),
                hyperlink: "".to_string(),
                note: "".to_string(),
                pivot_table: None,
                text_format_runs: vec![],
                user_entered_format: Some(CellFormat {
                    background_color: None,
                    background_color_style: None,
                    borders: None,
                    horizontal_alignment: Some(HorizontalAlignment::Center),
                    hyperlink_display_type: None,
                    number_format: None,
                    padding: None,
                    text_direction: None,
                    text_format: None,
                    text_rotation: None,
                    vertical_alignment: Some(VerticalAlignment::Middle),
                    wrap_strategy: Some(WrapStrategy::Wrap),
                }),
                user_entered_value: None,
            }),
            fields: "userEnteredFormat(horizontalAlignment, verticalAlignment, wrapStrategy)"
                .to_string(),
        }),
        set_basic_filter: None,
        set_data_validation: None,
        sort_range: None,
        text_to_columns: None,
        trim_whitespace: None,
        unmerge_cells: None,
        update_banding: None,
        update_borders: None,
        update_cells: None,
        update_chart_spec: None,
        update_conditional_format_rule: None,
        update_data_source: None,
        update_developer_metadata: None,
        update_dimension_group: None,
        update_dimension_properties: None,
        update_embedded_object_border: None,
        update_embedded_object_position: None,
        update_filter_view: None,
        update_named_range: None,
        update_protected_range: None,
        update_sheet_properties: None,
        update_slicer_spec: None,
        update_spreadsheet_properties: None,
    });

    let horizontally_size_separator_columns = (NUM_COLS_PER_PERSON..num_columns)
        .step_by(NUM_COLS_PER_PERSON as usize + 1)
        .map(|i| Request {
            add_banding: None,
            add_chart: None,
            add_conditional_format_rule: None,
            add_data_source: None,
            add_dimension_group: None,
            add_filter_view: None,
            add_named_range: None,
            add_protected_range: None,
            add_sheet: None,
            add_slicer: None,
            append_cells: None,
            append_dimension: None,
            auto_fill: None,
            auto_resize_dimensions: None,
            clear_basic_filter: None,
            copy_paste: None,
            create_developer_metadata: None,
            cut_paste: None,
            delete_banding: None,
            delete_conditional_format_rule: None,
            delete_data_source: None,
            delete_developer_metadata: None,
            delete_dimension: None,
            delete_dimension_group: None,
            delete_duplicates: None,
            delete_embedded_object: None,
            delete_filter_view: None,
            delete_named_range: None,
            delete_protected_range: None,
            delete_range: None,
            delete_sheet: None,
            duplicate_filter_view: None,
            duplicate_sheet: None,
            find_replace: None,
            insert_dimension: None,
            insert_range: None,
            merge_cells: None,
            move_dimension: None,
            paste_data: None,
            randomize_range: None,
            refresh_data_source: None,
            repeat_cell: None,
            set_basic_filter: None,
            set_data_validation: None,
            sort_range: None,
            text_to_columns: None,
            trim_whitespace: None,
            unmerge_cells: None,
            update_banding: None,
            update_borders: None,
            update_cells: None,
            update_chart_spec: None,
            update_conditional_format_rule: None,
            update_data_source: None,
            update_developer_metadata: None,
            update_dimension_group: None,
            update_dimension_properties: Some(UpdateDimensionPropertiesRequest {
                data_source_sheet_range: None,
                fields: "pixelSize".to_string(),
                properties: Some(DimensionProperties {
                    data_source_column_reference: None,
                    developer_metadata: vec![],
                    hidden_by_filter: false,
                    hidden_by_user: false,
                    pixel_size: 10,
                }),
                range: Some(DimensionRange {
                    dimension: Some(Dimension::Columns),
                    end_index: i as i64 + 1,
                    sheet_id: 0,
                    start_index: i as i64,
                }),
            }),
            update_embedded_object_border: None,
            update_embedded_object_position: None,
            update_filter_view: None,
            update_named_range: None,
            update_protected_range: None,
            update_sheet_properties: None,
            update_slicer_spec: None,
            update_spreadsheet_properties: None,
        });

    let merge_separator_columns = (NUM_COLS_PER_PERSON..num_columns)
        .step_by(NUM_COLS_PER_PERSON as usize + 1)
        .map(|i| Request {
            add_banding: None,
            add_chart: None,
            add_conditional_format_rule: None,
            add_data_source: None,
            add_dimension_group: None,
            add_filter_view: None,
            add_named_range: None,
            add_protected_range: None,
            add_sheet: None,
            add_slicer: None,
            append_cells: None,
            append_dimension: None,
            auto_fill: None,
            auto_resize_dimensions: None,
            clear_basic_filter: None,
            copy_paste: None,
            create_developer_metadata: None,
            cut_paste: None,
            delete_banding: None,
            delete_conditional_format_rule: None,
            delete_data_source: None,
            delete_developer_metadata: None,
            delete_dimension: None,
            delete_dimension_group: None,
            delete_duplicates: None,
            delete_embedded_object: None,
            delete_filter_view: None,
            delete_named_range: None,
            delete_protected_range: None,
            delete_range: None,
            delete_sheet: None,
            duplicate_filter_view: None,
            duplicate_sheet: None,
            find_replace: None,
            insert_dimension: None,
            insert_range: None,
            merge_cells: Some(MergeCellsRequest {
                merge_type: Some(MergeType::MergeColumns),
                range: Some(GridRange {
                    end_column_index: i as i64 + 1,
                    end_row_index: num_rows as i64,
                    sheet_id: 0,
                    start_column_index: i as i64,
                    start_row_index: 0,
                }),
            }),
            move_dimension: None,
            paste_data: None,
            randomize_range: None,
            refresh_data_source: None,
            repeat_cell: None,
            set_basic_filter: None,
            set_data_validation: None,
            sort_range: None,
            text_to_columns: None,
            trim_whitespace: None,
            unmerge_cells: None,
            update_banding: None,
            update_borders: None,
            update_cells: None,
            update_chart_spec: None,
            update_conditional_format_rule: None,
            update_data_source: None,
            update_developer_metadata: None,
            update_dimension_group: None,
            update_dimension_properties: None,
            update_embedded_object_border: None,
            update_embedded_object_position: None,
            update_filter_view: None,
            update_named_range: None,
            update_protected_range: None,
            update_sheet_properties: None,
            update_slicer_spec: None,
            update_spreadsheet_properties: None,
        });
    client
        .spreadsheets()
        .batch_update(
            spreadsheet_id,
            &BatchUpdateSpreadsheetRequest {
                include_spreadsheet_in_response: None,
                requests: horizontally_size_name_columns
                    .chain(vertically_size_rows)
                    .chain(horizontally_size_photos_columns)
                    .chain(vert_and_horiz_center_items_in_rows)
                    .chain(horizontally_size_separator_columns)
                    .chain(merge_separator_columns)
                    .collect(),

                response_include_grid_data: None,
                response_ranges: vec![],
            },
        )
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spreadsheet_size() {
        let (r, c) = size_of_spreadsheet(0);
        assert_eq!(r, 0);
        assert_eq!(c, 8);

        let (r, c) = size_of_spreadsheet(1);
        assert_eq!(r, 1);
        assert_eq!(c, 8);

        let (r, c) = size_of_spreadsheet(11);
        assert_eq!(r, 11);
        assert_eq!(c, 8);

        let (r, c) = size_of_spreadsheet(33);
        assert_eq!(r, 11);
        assert_eq!(c, 8);

        let (r, c) = size_of_spreadsheet(34);
        assert_eq!(r, 12);
        assert_eq!(c, 8);
    }
}
